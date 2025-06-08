extends Control


func _ready() -> void:
	%PlayButton.grab_focus()

func _on_play_button_pressed() -> void:
	Transitions.change_scene_with_transition("uid://bcyg1w7ev5m1i")
