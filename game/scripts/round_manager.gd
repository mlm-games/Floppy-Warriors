class_name RoundManager
extends Node

signal ranking_points_updated(amount: int)
signal score_changed(score: int)

signal round_started(
	round_number: int,
	enemies_total: int,
	is_boss: bool
)
signal round_progress_changed(
	round_number: int,
	enemies_remaining: int,
	enemies_total: int
)
signal round_cleared(round_number: int, recovered_health: int)

signal enemy_spawn_requested(config: Dictionary)
signal reward_choices_ready(choices: Array[Dictionary])
signal reward_applied(reward_id: StringName)

signal run_ended(
	victory: bool,
	round_reached: int,
	final_score: int,
	stats: Dictionary
)

const FINAL_ROUND: int = 15
const NEXT_ENEMY_DELAY: float = 1.0

const REWARD_POOL: Array[Dictionary] = [
	{
		"id": &"power",
		"title": "Sharpened Arrows",
		"description": "+25% arrow damage."
	},
	{
		"id": &"vitality",
		"title": "Reinforced Body",
		"description": "+25 maximum HP and heal the added HP."
	},
	{
		"id": &"quickdraw",
		"title": "Quick Draw",
		"description": "+20% bow draw speed."
	},
	{
		"id": &"velocity",
		"title": "Tighter String",
		"description": "+15% arrow velocity."
	},
	{
		"id": &"head_hunter",
		"title": "Head Hunter",
		"description": "+25% headshot damage."
	},
	{
		"id": &"quiver",
		"title": "Split Shot",
		"description": "Fire one additional arrow with spread."
	},
	{
		"id": &"knockback",
		"title": "Braced Limbs",
		"description": "+30% arrow knockback."
	},
	{
		"id": &"field_dressing",
		"title": "Field Dressing",
		"description": "Recover 45% of maximum HP."
	},
	{
		"id": &"acrobatics",
		"title": "Acrobatics",
		"description": "Reduce airdodge cooldown by 15%."
	}
]

var ranking_points: int = 0:
	set(value):
		ranking_points = value
		ranking_points_updated.emit(ranking_points)
		score_changed.emit(ranking_points)

var player: Player

var round_number: int = 0
var enemies_total: int = 0
var enemies_remaining: int = 0
var enemies_spawned: int = 0

var run_active: bool = false
var awaiting_reward: bool = false

var current_enemy_score_value: int = 0
var current_reward_choices: Array[Dictionary] = []

var stats: Dictionary = {
	"kills": 0,
	"headshots": 0,
	"damage_dealt": 0
}


func begin_run(player_reference: Player) -> void:
	player = player_reference

	round_number = 0
	enemies_total = 0
	enemies_remaining = 0
	enemies_spawned = 0

	ranking_points = 0
	run_active = true
	awaiting_reward = false
	current_reward_choices.clear()

	stats = {
		"kills": 0,
		"headshots": 0,
		"damage_dealt": 0
	}

	if (
		is_instance_valid(player)
		and not player.hit_confirmed.is_connected(
			_on_player_hit_confirmed
		)
	):
		player.hit_confirmed.connect(_on_player_hit_confirmed)

	# Apply permanent meta bonuses at the start of every run.
	if is_instance_valid(player):
		Meta.apply_to_player(player)

	start_next_round()


func reset() -> void:
	var found_player := get_tree().get_first_node_in_group(
		"Player"
	) as Player

	if found_player != null:
		begin_run(found_player)


func start_next_round() -> void:
	if not run_active or awaiting_reward:
		return

	round_number += 1
	enemies_total = _get_enemy_count_for_round(round_number)
	enemies_remaining = enemies_total
	enemies_spawned = 0

	round_started.emit(
		round_number,
		enemies_total,
		is_boss_round()
	)

	round_progress_changed.emit(
		round_number,
		enemies_remaining,
		enemies_total
	)

	_request_next_enemy()


func _get_enemy_count_for_round(value: int) -> int:
	if value % 5 == 0:
		return 1

	return mini(1 + int((value - 1) / 3), 4)


func is_boss_round() -> bool:
	return round_number > 0 and round_number % 5 == 0


func _request_next_enemy() -> void:
	if (
		not run_active
		or awaiting_reward
		or enemies_remaining <= 0
	):
		return

	enemies_spawned += 1

	var config := _build_enemy_config()
	current_enemy_score_value = int(config["score_value"])

	enemy_spawn_requested.emit(config)


func _build_enemy_config() -> Dictionary:
	var boss := is_boss_round()

	var health_multiplier := 1.0 + float(round_number - 1) * 0.14
	var damage_multiplier := 1.0 + float(round_number - 1) * 0.055
	var draw_multiplier := 1.0 + minf(
		0.45,
		float(round_number - 1) * 0.025
	)

	var aim_error := maxf(
		7.0,
		26.0 - float(round_number) * 1.2
	)

	var decision_minimum := maxf(
		0.75,
		1.6 - float(round_number) * 0.04
	)
	var decision_maximum := maxf(
		1.5,
		3.2 - float(round_number) * 0.06
	)

	if boss:
		health_multiplier *= 2.25
		damage_multiplier *= 1.15
		draw_multiplier *= 1.1
		aim_error *= 0.65
		decision_minimum *= 0.85
		decision_maximum *= 0.85

	return {
		"round": round_number,
		"enemy_index": enemies_spawned,
		"boss": boss,
		"health": roundi(100.0 * health_multiplier),
		"damage_multiplier": damage_multiplier,
		"draw_speed_multiplier": draw_multiplier,
		"aim_error": aim_error,
		"decision_delay_min": decision_minimum,
		"decision_delay_max": decision_maximum,
		"score_value": (
			50 + round_number * 10
			if boss
			else 10 + round_number * 2
		)
	}


func notify_enemy_defeated() -> void:
	if not run_active or enemies_remaining <= 0:
		return

	enemies_remaining -= 1
	stats["kills"] = int(stats["kills"]) + 1
	ranking_points += current_enemy_score_value

	round_progress_changed.emit(
		round_number,
		enemies_remaining,
		enemies_total
	)

	if enemies_remaining > 0:
		_queue_next_enemy()
		return

	if round_number >= FINAL_ROUND:
		end_run(true)
		return

	_finish_round()


func _queue_next_enemy() -> void:
	await get_tree().create_timer(NEXT_ENEMY_DELAY).timeout

	if run_active and not awaiting_reward and enemies_remaining > 0:
		_request_next_enemy()


func _finish_round() -> void:
	awaiting_reward = true

	var recovered_health := 0

	if is_instance_valid(player) and not player.is_dead:
		recovered_health = player.heal(
			maxi(5, roundi(player.get_max_health() * 0.12))
		)

	round_cleared.emit(round_number, recovered_health)

	current_reward_choices = _roll_reward_choices()
	reward_choices_ready.emit(current_reward_choices.duplicate(true))


func _roll_reward_choices() -> Array[Dictionary]:
	var pool: Array[Dictionary] = []

	for reward in REWARD_POOL:
		var reward_id := StringName(reward.get("id", &""))

		if (
			reward_id == &"quiver"
			and is_instance_valid(player)
			and player.arrow_count >= 5
		):
			continue

		pool.append(reward.duplicate(true))

	pool.shuffle()

	var result: Array[Dictionary] = []
	var choice_count := mini(3, pool.size())

	for index in range(choice_count):
		result.append(pool[index])

	return result


func choose_reward(reward_id: StringName) -> bool:
	if not run_active or not awaiting_reward:
		return false

	var valid_choice := false

	for choice in current_reward_choices:
		if StringName(choice.get("id", &"")) == reward_id:
			valid_choice = true
			break

	if not valid_choice:
		return false

	_apply_reward(reward_id)

	awaiting_reward = false
	current_reward_choices.clear()
	reward_applied.emit(reward_id)

	start_next_round.call_deferred()
	return true


func _apply_reward(reward_id: StringName) -> void:
	if not is_instance_valid(player):
		return

	match reward_id:
		&"power":
			player.arrow_damage_multiplier *= 1.25

		&"vitality":
			player.add_max_health(25, true)

		&"quickdraw":
			player.draw_speed_multiplier *= 1.2

		&"velocity":
			player.arrow_velocity_multiplier *= 1.15

		&"head_hunter":
			player.headshot_damage_multiplier *= 1.25

		&"quiver":
			player.arrow_count = mini(player.arrow_count + 1, 5)
			player.arrow_spread_degrees += 4.0

		&"knockback":
			player.arrow_knockback_multiplier *= 1.3

		&"field_dressing":
			player.heal(
				roundi(player.get_max_health() * 0.45)
			)

		&"acrobatics":
			player.airdodge_cooldown = maxf(
				0.35,
				player.airdodge_cooldown * 0.85
			)


func _on_player_hit_confirmed(
	limb_name: StringName,
	damage: int,
	_killed: bool
) -> void:
	stats["damage_dealt"] = int(stats["damage_dealt"]) + damage

	if limb_name == &"Head":
		stats["headshots"] = int(stats["headshots"]) + 1


func end_run(victory: bool = false) -> void:
	if not run_active:
		return

	run_active = false
	awaiting_reward = false
	current_reward_choices.clear()

	if victory:
		ranking_points += 250

	var final_stats := stats.duplicate(true)
	final_stats["round"] = round_number
	final_stats["score"] = ranking_points

	var bones_earned := Meta.award_run(
		victory,
		round_number,
		ranking_points,
		final_stats
	)
	final_stats["bones_earned"] = bones_earned
	final_stats["bones_total"] = Meta.bones

	run_ended.emit(
		victory,
		round_number,
		ranking_points,
		final_stats
	)