# blend 0.8

* Added better support for Blender primitives. Should be more resilient to future updates to the blend file.
* Changed formatting to print untyped arrays of primitives as a list of u8s.
* Updated Blend::from_path and Blend::new to return Results.
* Renamed Blend::get_all_root_blocks to Blend::root_instances.
* Renamed Blend::get_by_code to Blend::instances_with_code.
* Blend::root_instances and Blend::instances_with_code both return an impl Iterator<Item=Instance>.
* Added 4 example blend files from different Blender versions.


# blend 0.7

