# main.gd
extends Node2D

@export var ai_player_scene: PackedScene # Assign AIPlayer.tscn in Inspector
@export var player_start_position: Vector2 = Vector2(200, 450)
@export var ai_start_position: Vector2 = Vector2(800, 450)

@onready var player: Node2D = $Player
# @onready var ai_player: Node2D = $AIPlayer # We will spawn AI dynamically
@onready var status_label: Label = $UI/StatusLabel
@onready var player_health_label: Label = $UI/PlayerHealthLabel # New label for player health
@onready var ai_health_label: Label = $UI/AIHealthLabel # New label for AI health

var current_ai_player: Node2D = null
var game_over: bool = false
var score: int = 0


func _ready() -> void:
	if not is_instance_valid(player):
		printerr("Player instance not found in Main scene! Ensure it's named 'Player'.")
		return
	
	player.position = player_start_position
	player.died.connect(_on_player_died)
	if player.has_method("set_initial_health_reference"): # If player script tracks initial health
		player.set_initial_health_reference()

	spawn_new_ai()
	status_label.text = "Score: " + str(score)
	update_health_labels()


func _process(delta: float) -> void: # Use _process for UI updates
	if not game_over:
		update_health_labels()


func update_health_labels() -> void:
	if is_instance_valid(player) and player.has_method("get_current_health"):
		player_health_label.text = "Player HP: %d" % player.get_current_health()
	if is_instance_valid(current_ai_player) and current_ai_player.has_method("get_current_health"):
		ai_health_label.text = "Enemy HP: %d" % current_ai_player.get_current_health()
	else:
		ai_health_label.text = "Enemy HP: -"


func spawn_new_ai() -> void:
	if not ai_player_scene:
		printerr("AI Player Scene not assigned in Main script!")
		return

	if is_instance_valid(current_ai_player): # Should not happen if called correctly
		current_ai_player.queue_free()

	current_ai_player = ai_player_scene.instantiate()
	current_ai_player.name = "AIPlayerInstance" # Unique name
	add_child(current_ai_player)
	current_ai_player.position = ai_start_position
	
	current_ai_player.died.connect(_on_ai_player_died)
	if current_ai_player.has_method("set_target"):
		current_ai_player.set_target(player)
	if current_ai_player.has_method("set_initial_health_reference"):
		current_ai_player.set_initial_health_reference()
	
	# Optional: Increase difficulty slightly for new AI (e.g., more health or faster firing)
	# current_ai_player.health += score * 5 # Example


func _on_player_died() -> void:
	if game_over: return
	game_over = true
	status_label.text = "YOU LOSE!\nFinal Score: %d\nPress R to Restart" % score
	player_health_label.text = "Player HP: 0"
	if is_instance_valid(current_ai_player):
		if current_ai_player.has_method("stop_ai"): # Add this method to AIPlayer.gd
			current_ai_player.stop_ai()


func _on_ai_player_died() -> void:
	if game_over: return # Don't respawn if player already lost
	
	score += 1
	status_label.text = "Score: " + str(score)
	ai_health_label.text = "Enemy HP: -" # Clear while respawning

	if is_instance_valid(current_ai_player):
		current_ai_player.queue_free() # Remove the old AI
		current_ai_player = null
	
	# Spawn new AI after a short delay
	var spawn_timer = get_tree().create_timer(1.5) # 1.5 second delay
	spawn_timer.timeout.connect(spawn_new_ai)


func _input(event: InputEvent) -> void:
	if game_over and event.is_action_pressed("restart_game"):
		score = 0 # Reset score
		game_over = false
		get_tree().reload_current_scene()
