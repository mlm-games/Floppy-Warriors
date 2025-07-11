class_name BaseData extends Resource

var id: String:
	get:
		if resource_name != "":
			return resource_name
		if resource_path.is_empty():
			return "Resource path is empty or not saved..."
			
		var temp_id = resource_path.get_file().trim_suffix(".tres")
		if id == "": 
			id = temp_id
		else:
			if Engine.is_editor_hint():
				if temp_id != "" and id != temp_id:
					id = temp_id
		resource_name = temp_id
		return id

@export var enabled := true #NOTE: Should only be updated in game / not saved? nvm use a different dict for it
@export var unlocked := true
