class_name ArrowData
extends BaseData

@export var damage: float = 20.0

@export_group("Limb Damage")
@export var head_damage_multiplier: float = 2.0
@export var torso_damage_multiplier: float = 1.0
@export var arm_damage_multiplier: float = 0.75
@export var leg_damage_multiplier: float = 0.65

@export_group("Physics")
@export var knockback_multiplier: float = 1.0
@export var stuck_lifetime: float = 2.0


func get_limb_damage_multiplier(limb_name: StringName) -> float:
	match limb_name:
		&"Head":
			return head_damage_multiplier

		&"Torso":
			return torso_damage_multiplier

		&"LowerArmL", &"LowerArmR":
			return arm_damage_multiplier

		&"LowerLegL", &"LowerLegR":
			return leg_damage_multiplier

		_:
			return torso_damage_multiplier