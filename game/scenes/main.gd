extends Node2D

@export var ai_player_scene: PackedScene
@export var player_start_position: Vector2 = Vector2(200, 450)
@export var ai_start_position: Vector2 = Vector2(800, 450)

@onready var player: Player = $Player
@onready var status_label: Label = $UI/StatusLabel
@onready var player_health_label: Label = %PlayerHealthLabel
@onready var ai_health_label: Label = $UI/AIHealthLabel
@onready var ui_layer: CanvasLayer = $UI
@onready var game_camera: Camera2D = %Camera2D

var round_manager: RoundManager
var current_ai_player: EnemyAI

var reward_overlay: Control
var game_over: bool = false
var camera_tween: Tween


func _ready() -> void:
	if not is_instance_valid(player):
		printerr(
			"Player instance not found in Main scene. "
			+ "Ensure it is named Player."
		)
		return

	player.position = player_start_position
	player.died.connect(_on_player_died)
	player.set_initial_health_reference()

	round_manager = RoundManager.new()
	round_manager.name = "RoundManager"
	add_child(round_manager)

	round_manager.round_started.connect(_on_round_started)
	round_manager.round_progress_changed.connect(
		_on_round_progress_changed
	)
	round_manager.enemy_spawn_requested.connect(spawn_new_ai)
	round_manager.reward_choices_ready.connect(
		_show_reward_choices
	)
	round_manager.score_changed.connect(_on_score_changed)
	round_manager.run_ended.connect(_on_run_ended)

	update_health_labels()
	round_manager.begin_run(player)


func _process(_delta: float) -> void:
	if not game_over:
		update_health_labels()
		game_camera.offset = game_camera.offset.lerp(
			Vector2.ZERO,
			0.1
		)


func update_health_labels() -> void:
	if is_instance_valid(player):
		player_health_label.text = (
			"Player HP: %d/%d"
			% [
				player.get_current_health(),
				player.get_max_health()
			]
		)

	if is_instance_valid(current_ai_player):
		ai_health_label.text = (
			"Enemy HP: %d/%d"
			% [
				current_ai_player.get_current_health(),
				current_ai_player.get_max_health()
			]
		)
	else:
		ai_health_label.text = "Enemy HP: -"


func spawn_new_ai(config: Dictionary) -> void:
	if game_over:
		return

	if is_instance_valid(current_ai_player):
		push_warning("Tried to spawn an AI while another AI was active.")
		return

	if not ai_player_scene:
		printerr("AI Player Scene not assigned in Main script!")
		round_manager.end_run(false)
		return

	var instance := ai_player_scene.instantiate()

	if not instance is EnemyAI:
		printerr("Assigned AI scene root is not EnemyAI.")
		instance.queue_free()
		round_manager.end_run(false)
		return

	current_ai_player = instance as EnemyAI

	var player_x := player.torso.global_position.x
	var spawn_x := ai_start_position.x

	if player_x < 600.0:
		spawn_x = randf_range(700.0, 1050.0)
	else:
		spawn_x = randf_range(150.0, 500.0)

	current_ai_player.position = Vector2(
		spawn_x,
		ai_start_position.y
	)

	add_child(current_ai_player)

	current_ai_player.died.connect(_on_ai_player_died)
	current_ai_player.set_target(player)
	current_ai_player.configure_for_round(config)

	if bool(config.get("boss", false)):
		current_ai_player.modulate = Color(1.0, 0.65, 0.5)
	else:
		current_ai_player.modulate = Color.WHITE

	update_health_labels()


func _on_player_died() -> void:
	if game_over:
		return

	if is_instance_valid(current_ai_player):
		current_ai_player.stop_ai_timers()

	tween_camera_on_player_death()
	round_manager.end_run(false)


func _on_ai_player_died() -> void:
	if game_over:
		return

	var defeated_enemy := current_ai_player
	current_ai_player = null
	ai_health_label.text = "Enemy HP: -"

	if is_instance_valid(defeated_enemy):
		defeated_enemy.call_queue_free()

	round_manager.notify_enemy_defeated()


func _on_round_started(
	round_number: int,
	enemies_in_round: int,
	is_boss: bool
) -> void:
	if is_boss:
		status_label.text = (
			"BOSS ROUND %d/%d\nScore: %d"
			% [
				round_number,
				RoundManager.FINAL_ROUND,
				round_manager.ranking_points
			]
		)
	else:
		status_label.text = (
			"Round %d/%d — %d enemies\nScore: %d"
			% [
				round_number,
				RoundManager.FINAL_ROUND,
				enemies_in_round,
				round_manager.ranking_points
			]
		)


func _on_round_progress_changed(
	round_number: int,
	enemies_remaining: int,
	enemies_total: int
) -> void:
	if round_manager.is_boss_round():
		status_label.text = (
			"BOSS ROUND %d/%d\nScore: %d"
			% [
				round_number,
				RoundManager.FINAL_ROUND,
				round_manager.ranking_points
			]
		)
	else:
		status_label.text = (
			"Round %d/%d — Enemies: %d/%d\nScore: %d"
			% [
				round_number,
				RoundManager.FINAL_ROUND,
				enemies_remaining,
				enemies_total,
				round_manager.ranking_points
			]
		)


func _on_score_changed(_score: int) -> void:
	if (
		not game_over
		and not round_manager.awaiting_reward
	):
		_on_round_progress_changed(
			round_manager.round_number,
			round_manager.enemies_remaining,
			round_manager.enemies_total
		)


func _show_reward_choices(choices: Array[Dictionary]) -> void:
	player.set_input_enabled(false)
	_clear_reward_overlay()

	status_label.text = (
		"Round %d cleared!\nChoose an upgrade"
		% round_manager.round_number
	)

	var background := ColorRect.new()
	background.name = "RewardOverlay"
	background.color = Color(0.02, 0.03, 0.05, 0.88)
	background.mouse_filter = Control.MOUSE_FILTER_STOP
	background.z_index = 100

	ui_layer.add_child(background)
	background.set_anchors_and_offsets_preset(
		Control.PRESET_FULL_RECT
	)

	reward_overlay = background

	var center := CenterContainer.new()
	background.add_child(center)
	center.set_anchors_and_offsets_preset(
		Control.PRESET_FULL_RECT
	)

	var panel := PanelContainer.new()
	panel.custom_minimum_size = Vector2(900, 380)
	center.add_child(panel)

	var vertical_box := VBoxContainer.new()
	vertical_box.add_theme_constant_override("separation", 20)
	panel.add_child(vertical_box)

	var title := Label.new()
	title.text = "Choose a Reward"
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title.add_theme_font_size_override("font_size", 32)
	vertical_box.add_child(title)

	var subtitle := Label.new()
	subtitle.text = "Your health recovered slightly between rounds."
	subtitle.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vertical_box.add_child(subtitle)

	var card_row := HBoxContainer.new()
	card_row.alignment = BoxContainer.ALIGNMENT_CENTER
	card_row.add_theme_constant_override("separation", 18)
	vertical_box.add_child(card_row)

	var first_button: Button

	for choice in choices:
		var button := Button.new()
		var reward_id := StringName(choice.get("id", &""))
		var reward_title := str(choice.get("title", "Upgrade"))
		var description := str(choice.get("description", ""))

		button.custom_minimum_size = Vector2(260, 190)
		button.text = "%s\n\n%s" % [reward_title, description]
		button.pressed.connect(
			_on_reward_selected.bind(reward_id)
		)

		card_row.add_child(button)

		if first_button == null:
			first_button = button

	if first_button != null:
		first_button.grab_focus()


func _on_reward_selected(reward_id: StringName) -> void:
	if not round_manager.choose_reward(reward_id):
		return

	_clear_reward_overlay()
	player.set_input_enabled(true)


func _clear_reward_overlay() -> void:
	if is_instance_valid(reward_overlay):
		reward_overlay.queue_free()

	reward_overlay = null


func _on_run_ended(
	victory: bool,
	round_reached: int,
	final_score: int,
	stats: Dictionary
) -> void:
	game_over = true

	player.set_input_enabled(false)
	player.cancel_bow_draw()
	_clear_reward_overlay()

	if is_instance_valid(current_ai_player):
		current_ai_player.stop_ai_timers()

	var kills := int(stats.get("kills", 0))
	var headshots := int(stats.get("headshots", 0))
	var damage := int(stats.get("damage_dealt", 0))

	if victory:
		status_label.text = (
			"RUN COMPLETE!\n"
			+ "Score: %d | Kills: %d\n"
			+ "Headshots: %d | Damage: %d\n"
			+ "Press R or Click to Restart"
		) % [
			final_score,
			kills,
			headshots,
			damage
		]
	else:
		status_label.text = (
			"YOU LOSE!\n"
			+ "Reached round %d/%d\n"
			+ "Score: %d | Kills: %d\n"
			+ "Headshots: %d | Damage: %d\n"
			+ "Press R or Click to Restart"
		) % [
			round_reached,
			RoundManager.FINAL_ROUND,
			final_score,
			kills,
			headshots,
			damage
		]

	update_health_labels()


func _input(event: InputEvent) -> void:
	if (
		game_over
		and (
			event.is_action_pressed("restart_game")
			or event.is_action_pressed("fire_bow")
		)
	):
		get_tree().reload_current_scene()
		return

	if event.is_action_pressed("settings"):
		STransitions.change_scene_with_transition(
			"uid://dp42fom7cc3n0"
		)


func tween_camera_on_player_death() -> void:
	if camera_tween:
		camera_tween.kill()

	camera_tween = get_tree().create_tween()
	camera_tween.set_ease(Tween.EASE_IN_OUT)
	camera_tween.set_trans(Tween.TRANS_CUBIC)
	camera_tween.tween_property(
		game_camera,
		"offset:y",
		1600.0,
		2.0
	)