extends Control

var health_bar: ProgressBar
var health_label: Label

func _ready():
	set_anchors_and_offsets_preset(Control.PRESET_TOP_LEFT)
	position = Vector2(20, 20)
	
	# Create background
	var bg = ColorRect.new()
	bg.size = Vector2(200, 30)
	bg.color = Color(0, 0, 0, 0.5)
	add_child(bg)
	
	# Create health bar
	health_bar = ProgressBar.new()
	health_bar.size = Vector2(190, 20)
	health_bar.position = Vector2(5, 5)
	health_bar.max_value = 100
	health_bar.value = 100
	health_bar.show_percentage = false
	add_child(health_bar)
	
	# Style the health bar
	var style_bg = StyleBoxFlat.new()
	style_bg.bg_color = Color(0.2, 0.2, 0.2)
	health_bar.add_theme_stylebox_override("background", style_bg)
	
	var style_fill = StyleBoxFlat.new()
	style_fill.bg_color = Color(0.8, 0.2, 0.2)
	health_bar.add_theme_stylebox_override("fill", style_fill)
	
	# Create label
	health_label = Label.new()
	health_label.text = "Health: 100"
	health_label.position = Vector2(210, 5)
	add_child(health_label)

func update_health(health: int):
	health_bar.value = health
	health_label.text = "Health: " + str(health)
	
	# Change color based on health
	var style_fill = health_bar.get_theme_stylebox("fill")
	if health > 60:
		style_fill.bg_color = Color(0.2, 0.8, 0.2)
	elif health > 30:
		style_fill.bg_color = Color(0.8, 0.8, 0.2)
	else:
		style_fill.bg_color = Color(0.8, 0.2, 0.2)
