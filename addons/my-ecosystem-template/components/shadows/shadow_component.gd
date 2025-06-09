extends Node2D
class_name ShadowSilhouette

@export var shadow_color = Color(0.141, 0.122, 0.192, 0.4)
@export var shadow_offset = Vector2(4, 6)
@export var shadow_scale = Vector2(1.1, 0.6)
@export var shadow_skew = 0.2

func create_shadow(target: Node2D) -> ShadowSilhouette:
	var shadow = Node2D.new()
	shadow.name = "Shadow"
	shadow.z_index = -1
	
	# Copy visual elements
	for child in target.get_children():
		if child is Polygon2D:
			var shadow_poly = Polygon2D.new()
			shadow_poly.polygon = child.polygon
			shadow_poly.color = shadow_color
			shadow_poly.position = child.position
			shadow_poly.rotation = child.rotation
			shadow.add_child(shadow_poly)
	
	# Apply shadow transform
	shadow.position = shadow_offset
	shadow.scale = shadow_scale
	shadow.skew = shadow_skew
	
	return shadow
