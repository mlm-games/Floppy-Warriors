class_name ShadowComponent
extends Node2D

@export var shadow_distance_multiplier = 1.0
@export var max_shadow_distance = 100.0
@export var ground_y = 500.0

var shadows = {}
var shadow_container: Node2D

func _ready():
	shadow_container = Node2D.new()
	shadow_container.name = "Shadows"
	shadow_container.z_index = -10
	add_child(shadow_container)
	
	# Create shadows for each body part
	for child in get_parent().get_children():
		if child is RigidBody2D:
			create_body_part_shadow(child)

func create_body_part_shadow(body_part: RigidBody2D):
	var shadow = Sprite2D.new()
	shadow.modulate = Color(0, 0, 0, 0.3)
	
	# Create ellipse shadow texture
	var shadow_texture = GradientTexture2D.new()
	shadow_texture.gradient = create_shadow_gradient()
	shadow_texture.width = 64
	shadow_texture.height = 32
	shadow_texture.fill = GradientTexture2D.FILL_RADIAL
	shadow_texture.fill_from = Vector2(0.5, 0.5)
	shadow_texture.fill_to = Vector2(1.0, 0.5)
	
	shadow.texture = shadow_texture
	shadow_container.add_child(shadow)
	shadows[body_part] = shadow

func create_shadow_gradient() -> Gradient:
	var gradient = Gradient.new()
	gradient.set_color(0, Color(0, 0, 0, 0.4))
	gradient.set_color(1, Color(0, 0, 0, 0))
	return gradient

func _physics_process(delta):
	for body_part in shadows:
		var shadow = shadows[body_part]
		if not is_instance_valid(body_part):
			continue
			
		# Calculate shadow position
		var height = ground_y - body_part.global_position.y
		height = clamp(height, 0, max_shadow_distance)
		
		shadow.global_position.x = body_part.global_position.x
		shadow.global_position.y = ground_y
		
		# Scale shadow based on height
		var scale_factor = 1.0 - (height / max_shadow_distance) * 0.5
		shadow.scale = Vector2(scale_factor, scale_factor * 0.5)
		
		# Fade shadow based on height
		var alpha = 0.3 * (1.0 - height / max_shadow_distance)
		shadow.modulate.a = alpha
