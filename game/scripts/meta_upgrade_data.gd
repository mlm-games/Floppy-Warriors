class_name MetaUpgradeData
extends BaseData
## Permanent upgrade definition. Levels live in Meta.meta_levels[id].

@export var display_name: String = "Upgrade"
@export_multiline var description: String = ""
@export var base_cost: int = 50
@export var cost_growth: float = 1.65
@export var max_level: int = 10

## Which player field this touches. Handled in Meta.apply_to_player().
@export_enum(
	"max_hp",
	"damage",
	"draw_speed",
	"headshot",
	"knockback",
	"velocity",
	"airdodge_cd",
	"bones_mult",
	"starting_arrow_count"
) var effect_type: String = "max_hp"

@export var amount_per_level: float = 1.0


func cost_for_level(current_level: int) -> int:
	return int(round(base_cost * pow(cost_growth, current_level)))