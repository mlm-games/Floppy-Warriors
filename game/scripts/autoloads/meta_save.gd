extends Node
## Autoload: Meta
## Persistent bones currency + permanent upgrade levels.

signal bones_changed(amount: int)
signal upgrade_purchased(id: StringName, new_level: int)
signal save_loaded()

const SAVE_PATH := "user://floppy_meta.json"
const SAVE_VERSION := 1

var bones: int = 0:
	set(value):
		bones = maxi(0, value)
		bones_changed.emit(bones)

var meta_levels: Dictionary = {} # StringName/String -> int
var best_round: int = 0
var total_runs: int = 0
var total_kills: int = 0
var total_victories: int = 0
var last_played_unix: int = 0
var last_run_bones_earned: int = 0

## Built-in catalog. Swap to .tres folder later if you want designer-facing data.
var CATALOG: Array[Dictionary] = [
	{
		"id": &"vitality",
		"display_name": "Vitality",
		"description": "+10 max HP per level",
		"base_cost": 40,
		"cost_growth": 1.55,
		"max_level": 12,
		"effect_type": "max_hp",
		"amount_per_level": 10.0,
	},
	{
		"id": &"power",
		"display_name": "Power",
		"description": "+6% arrow damage per level",
		"base_cost": 55,
		"cost_growth": 1.6,
		"max_level": 10,
		"effect_type": "damage",
		"amount_per_level": 0.06,
	},
	{
		"id": &"quickdraw",
		"display_name": "Quickdraw Drills",
		"description": "+5% draw speed per level",
		"base_cost": 50,
		"cost_growth": 1.58,
		"max_level": 10,
		"effect_type": "draw_speed",
		"amount_per_level": 0.05,
	},
	{
		"id": &"head_hunter",
		"display_name": "Head Hunter",
		"description": "+8% headshot damage per level",
		"base_cost": 70,
		"cost_growth": 1.7,
		"max_level": 8,
		"effect_type": "headshot",
		"amount_per_level": 0.08,
	},
	{
		"id": &"impact",
		"display_name": "Impact Training",
		"description": "+8% knockback per level",
		"base_cost": 45,
		"cost_growth": 1.55,
		"max_level": 8,
		"effect_type": "knockback",
		"amount_per_level": 0.08,
	},
	{
		"id": &"bowstring",
		"display_name": "Bowstring Oil",
		"description": "+4% arrow velocity per level",
		"base_cost": 45,
		"cost_growth": 1.55,
		"max_level": 8,
		"effect_type": "velocity",
		"amount_per_level": 0.04,
	},
	{
		"id": &"acrobat",
		"display_name": "Acrobat",
		"description": "-6% airdodge cooldown per level",
		"base_cost": 60,
		"cost_growth": 1.65,
		"max_level": 6,
		"effect_type": "airdodge_cd",
		"amount_per_level": 0.06,
	},
	{
		"id": &"fortune",
		"display_name": "Bone Fortune",
		"description": "+12% bones earned per level",
		"base_cost": 80,
		"cost_growth": 1.75,
		"max_level": 5,
		"effect_type": "bones_mult",
		"amount_per_level": 0.12,
	},
	{
		"id": &"multishot",
		"display_name": "Starting Quiver",
		"description": "+1 starting arrow every 2 levels (max +2)",
		"base_cost": 200,
		"cost_growth": 2.1,
		"max_level": 4,
		"effect_type": "starting_arrow_count",
		"amount_per_level": 0.5, # 2 levels = +1 arrow
	},
]


func _ready() -> void:
	load_game()
	_grant_offline_bones()
	save_loaded.emit()


func get_level(id: StringName) -> int:
	return int(meta_levels.get(String(id), 0))


func get_def(id: StringName) -> Dictionary:
	for entry in CATALOG:
		if StringName(entry["id"]) == id:
			return entry
	return {}


func get_cost(id: StringName) -> int:
	var def := get_def(id)
	if def.is_empty():
		return 999999
	var lvl := get_level(id)
	if lvl >= int(def["max_level"]):
		return 0
	return int(round(float(def["base_cost"]) * pow(float(def["cost_growth"]), lvl)))


func can_buy(id: StringName) -> bool:
	var def := get_def(id)
	if def.is_empty():
		return false
	var lvl := get_level(id)
	if lvl >= int(def["max_level"]):
		return false
	return bones >= get_cost(id)


func buy(id: StringName) -> bool:
	if not can_buy(id):
		return false
	var cost := get_cost(id)
	bones -= cost
	var new_level := get_level(id) + 1
	meta_levels[String(id)] = new_level
	save_game()
	upgrade_purchased.emit(id, new_level)
	return true


func apply_to_player(player: Player) -> void:
	if not is_instance_valid(player):
		return

	# Flat HP first so set_max_health works from a clean base.
	var bonus_hp := get_level(&"vitality") * 10
	if bonus_hp > 0:
		player.set_max_health(player.get_max_health() + bonus_hp, true)
		player.heal(player.get_max_health()) # full heal into new max at run start

	player.arrow_damage_multiplier *= 1.0 + get_level(&"power") * 0.06
	player.draw_speed_multiplier *= 1.0 + get_level(&"quickdraw") * 0.05
	player.headshot_damage_multiplier *= 1.0 + get_level(&"head_hunter") * 0.08
	player.arrow_knockback_multiplier *= 1.0 + get_level(&"impact") * 0.08
	player.arrow_velocity_multiplier *= 1.0 + get_level(&"bowstring") * 0.04

	var cd_cut := get_level(&"acrobat") * 0.06
	player.airdodge_cooldown = maxf(0.35, player.airdodge_cooldown * (1.0 - cd_cut))

	var extra_arrows := int(get_level(&"multishot") / 2)
	if extra_arrows > 0:
		player.arrow_count = mini(player.arrow_count + extra_arrows, 5)
		player.arrow_spread_degrees += 4.0 * float(extra_arrows)


func bones_multiplier() -> float:
	return 1.0 + get_level(&"fortune") * 0.12


func calculate_run_bones(
	victory: bool,
	round_reached: int,
	score: int,
	stats: Dictionary
) -> int:
	var kills := int(stats.get("kills", 0))
	var headshots := int(stats.get("headshots", 0))

	var raw := (
		kills * 3
		+ headshots * 4
		+ round_reached * 6
		+ int(score / 8.0)
		+ (120 if victory else 0)
	)
	return maxi(1, int(round(raw * bones_multiplier())))


func award_run(
	victory: bool,
	round_reached: int,
	score: int,
	stats: Dictionary
) -> int:
	var earned := calculate_run_bones(victory, round_reached, score, stats)
	last_run_bones_earned = earned
	bones += earned

	total_runs += 1
	total_kills += int(stats.get("kills", 0))
	best_round = maxi(best_round, round_reached)
	if victory:
		total_victories += 1

	save_game()
	return earned


func _grant_offline_bones() -> void:
	## Light idle drip so returning players see progression.
	## Unlocked by having any fortune OR vitality level (always-on tiny drip after first run).
	if last_played_unix <= 0 or total_runs <= 0:
		return

	var elapsed := int(Time.get_unix_time_from_system()) - last_played_unix
	elapsed = clampi(elapsed, 0, 60 * 60 * 6) # cap 6h
	var per_minute := 1 + get_level(&"fortune") # 1–6 bones/min
	var earned := int(elapsed / 60.0) * per_minute
	if earned > 0:
		bones += earned
		get_tree().set_meta("offline_bones", earned)


func save_game() -> void:
	last_played_unix = int(Time.get_unix_time_from_system())
	var payload := {
		"v": SAVE_VERSION,
		"bones": bones,
		"meta": meta_levels,
		"best_round": best_round,
		"total_runs": total_runs,
		"total_kills": total_kills,
		"total_victories": total_victories,
		"ts": last_played_unix,
	}
	var file := FileAccess.open(SAVE_PATH, FileAccess.WRITE)
	if file == null:
		push_error("Meta: failed to write save: %s" % FileAccess.get_open_error())
		return
	file.store_string(JSON.stringify(payload))


func load_game() -> void:
	if not FileAccess.file_exists(SAVE_PATH):
		return
	var text := FileAccess.get_file_as_string(SAVE_PATH)
	var data = JSON.parse_string(text)
	if typeof(data) != TYPE_DICTIONARY:
		return
	bones = int(data.get("bones", 0))
	meta_levels = data.get("meta", {})
	if typeof(meta_levels) != TYPE_DICTIONARY:
		meta_levels = {}
	best_round = int(data.get("best_round", 0))
	total_runs = int(data.get("total_runs", 0))
	total_kills = int(data.get("total_kills", 0))
	total_victories = int(data.get("total_victories", 0))
	last_played_unix = int(data.get("ts", 0))


func reset_all_progress() -> void:
	bones = 0
	meta_levels.clear()
	best_round = 0
	total_runs = 0
	total_kills = 0
	total_victories = 0
	last_run_bones_earned = 0
	save_game()