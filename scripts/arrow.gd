extends RigidBody2D

@export var arrow_data: ArrowData = preload("uid://cutbot3ne5uq3")

var shooter: Node = null # Who shot this arrow
var has_hit: bool = false

var vanish_tween: Tween

@onready var lifespan_timer: Timer = %LifespanTimer
@onready var groove_joint_2d: GrooveJoint2D = %GrooveJoint2D

func _ready() -> void:
	body_entered.connect(_on_body_entered)
	lifespan_timer.timeout.connect(call_queue_free)

func _physics_process(_delta: float) -> void:
	if not has_hit and linear_velocity.length() > 0.1:
		rotation = linear_velocity.angle()

func set_shooter(s: Node) -> void:
	shooter = s

func _on_body_entered(body: Node) -> void:
	if has_hit: return
	if body == shooter: return # Prevent hitting self (immediately)

	has_hit = true
	set_collision_layer_value(C.CollisionLayers.Arrows, false)
	set_collision_mask_value(C.CollisionLayers.PlayersActive, false)
	set_collision_mask_value(C.CollisionLayers.World, false)

	#var current_parent = get_parent()
	#if current_parent:
		#current_parent.remove_child.call_deferred(self)
	#body.add_child.call_deferred(self) # Stick arrow to the body it hit TODO: Pin a joint strictly without movement between the body and the arrow?
	#groove_joint_2d.node_b = body.get_path()
	#mass = 0.01

	if body.get_owner() is BaseWarrior:
			var target_root : BaseWarrior = body.get_owner() # To get the scene root of the limb
			if target_root != shooter:
				# Pass normalized velocity of the arrow for knockback direction
				@warning_ignore("narrowing_conversion")
				target_root.take_damage(arrow_data.damage, linear_velocity.normalized()) 

	# Lower the lifespan timer as it's now stuck and is harder for the player to move)
	lifespan_timer.start(2)

func call_queue_free()  -> void:
	if vanish_tween:
		vanish_tween.kill()
	
	vanish_tween = get_tree().create_tween().set_ease(Tween.EASE_IN_OUT).set_trans(Tween.TRANS_CUBIC)
	
	#vanish_tween.tween_interval(2)
	vanish_tween.tween_property(self, "modulate", Color.TRANSPARENT, 0.75)
	vanish_tween.tween_callback(queue_free)
