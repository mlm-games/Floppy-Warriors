extends Node2D

@export var ai_player_scene: PackedScene # Assign AIPlayer.tscn in Inspector
@export var player_start_position: Vector2 = Vector2(200, 450)
@export var ai_start_position: Vector2 = Vector2(800, 450)

@onready var player: Player = $Player
@onready var status_label: Label = $UI/StatusLabel
@onready var player_health_label: Label = %PlayerHealthLabel
@onready var player_stamina_label: Label = %PlayerStaminaLabel
@onready var ai_health_label: Label = $UI/AIHealthLabel

var current_ai_player: EnemyAI = null
var game_over: bool = false
var score: int = 0
var camera_tween : Tween

func _ready() -> void:
	#%Camera2D.global_position = %Camera2D.get_viewport_rect().size / 2
	if not is_instance_valid(player):
		printerr("Player instance not found in Main scene! Ensure it's named 'Player'.")
		return
	
	player.position = player_start_position
	player.died.connect(_on_player_died)
	player.set_initial_health_reference()

	spawn_new_ai()
	status_label.text = "Score: " + str(score)
	update_health_labels()


func _process(delta: float) -> void: # Use _process for UI updates
	if not game_over:
		update_health_labels()
		%Camera2D.offset = lerp(%Camera2D.offset, Vector2.ZERO, 0.1)
	#if player.global_position == %Camera2D.position:
		#(%Player.camera_2d as Camera2D).enabled = true; %Camera2D.enabled = false


func update_health_labels() -> void:
	player_health_label.text = "Player HP: %d" % player.get_current_health()
	if is_instance_valid(current_ai_player):
		ai_health_label.text = "Enemy HP: %d" % current_ai_player.get_current_health()
	else:
		ai_health_label.text = "Enemy HP: -"


func spawn_new_ai() -> void:
	if not ai_player_scene:
		printerr("AI Player Scene not assigned in Main script!")
		return

	current_ai_player = ai_player_scene.instantiate()
	add_child(current_ai_player)
	current_ai_player.position = Vector2(randi_range(100, 1000), 450) # Random spwan point
	
	current_ai_player.died.connect(_on_ai_player_died)
	current_ai_player.set_target(player)
	current_ai_player.set_initial_health_reference()
	
	current_ai_player.health += score * 0.5 # Increase difficulty slightly for new AI (e.g., more health or faster firing), For now leave at this


func _on_player_died() -> void:
	if game_over: return
	game_over = true
	tween_camera_on_player_death()
	status_label.text = "YOU LOSE!\nFinal Score: %d\nPress R or Click to Restart" % score
	player_health_label.text = "Player HP: 0"
	if is_instance_valid(current_ai_player):
		current_ai_player.stop_ai_timers()


func _on_ai_player_died() -> void:
	if game_over: return # Don't respawn if player already lost
	
	score += 10
	status_label.text = "Score: " + str(score)
	ai_health_label.text = "Enemy HP: -" # Clear while respawning

	if is_instance_valid(current_ai_player):
		current_ai_player.call_queue_free() # Remove the old AI
		current_ai_player = null
	
	# Spawn new AI after a short delay
	var spawn_timer = get_tree().create_timer(1.5) # 1.5 second delay
	spawn_timer.timeout.connect(spawn_new_ai)


func _input(event: InputEvent) -> void:
	if game_over and (event.is_action_pressed("restart_game") or event.is_action_pressed("fire_bow")):
		score = 0 # Reset score
		game_over = false
		get_tree().reload_current_scene()
	
	if event.is_action_pressed("settings"):
		Transitions.change_scene_with_transition("uid://dp42fom7cc3n0")


func tween_camera_on_player_death()  -> void:
	if camera_tween:
		camera_tween.kill()
	
	#if game_over: await get_tree().create_timer(2).timeout
	camera_tween = get_tree().create_tween().set_ease(Tween.EASE_IN_OUT).set_trans(Tween.TRANS_CUBIC)
	
	camera_tween.tween_property(%Camera2D, "offset:y", 1600, 2)
