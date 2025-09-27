import bpy
import os
import mathutils
import math
from bpy_extras.io_utils import axis_conversion

def convert_y_up(matrix):
    """Convert Blender’s Z-up coordinate system to OpenGL’s Y-up system."""
    conversion_matrix = mathutils.Matrix((
        (1,  0,  0,  0),
        (0,  0,  1,  0),
        (0, -1,  0,  0),
        (0,  0,  0,  1)
    ))
    return conversion_matrix @ matrix

def convert_y_up_quaternion(blender_quaternion):
    """
    Converts a Blender quaternion to an OpenGL-compatible quaternion.

    Args:
        blender_quaternion: A mathutils.Quaternion representing the rotation in Blender's coordinate system.

    Returns:
        A mathutils.Quaternion representing the rotation in OpenGL's coordinate system.
    """
    mat_convert = axis_conversion(from_forward='-Y', from_up='Z', to_forward='-Z', to_up='Y').to_4x4()

    q_convert = mat_convert.to_quaternion()
    q_new = q_convert @ blender_quaternion @ q_convert.inverted()

    return q_new

def export_animation_data(filepath):
    with open(filepath, "w") as f:
        f.write("# WiseModel 0.0.1\n")

        armatures = [obj for obj in bpy.context.selected_objects if obj.type == 'ARMATURE']
        if not armatures:
            print("No armature selected for export.")
            return

        armature = armatures[0]  # Assuming one armature per model
        f.write(f"BONECOUNT: {len(armature.pose.bones)}\n")
        

        #WARNING: This is kinda destructive, it changes the axis_conversion for the blender file.
        conv = axis_conversion(from_forward='-Y', from_up='Z', to_forward='-Z', to_up='Y').to_4x4()
        armature.data.transform(conv)
        
        fps = bpy.context.scene.render.fps
        f.write(f"FPS: {fps}\n")
        global_transform = armature.matrix_world.copy().inverted().transposed()
        f.write(f"GLOBAL_TRANSFORM:\n")
        for row in global_transform:
            f.write(f"{row[0]:.5f} {row[1]:.5f} {row[2]:.5f} {row[3]:.5f}\n")
        f.write("\n")
    
        for bone in armature.pose.bones:
            current_frame = bpy.context.scene.frame_current
            bpy.context.scene.frame_set(0)
            
            parent_index = -1 if bone.parent is None else list(armature.pose.bones).index(bone.parent)
            f.write(f"BONE_NAME: {bone.name}\nPARENT_INDEX: {parent_index}\nOFFSET_MATRIX:\n")
            
            # inverse bindpose matrix for the bone.
            
            # offset_matrix = convert_y_up(armature.matrix_world.copy() @ bone.bone.matrix_local).transposed().inverted_safe();
            offset_matrix = bone.bone.matrix_local.inverted().transposed()
            
            for row in offset_matrix:
                f.write(f"{row[0]:.5f} {row[1]:.5f} {row[2]:.5f} {row[3]:.5f}\n")
            f.write("\n")

        for action in bpy.data.actions:  # Iterate over all actions
            armature.animation_data.action = action  # Temporarily assign action to the armature

            if armature.animation_data and armature.animation_data.action:
                action = armature.animation_data.action
            
                f.write(f"ANIMATION_NAME: {action.name}\n")

                frame_start = int(action.frame_range[0])
                frame_end = int(action.frame_range[1])
                duration = (frame_end - frame_start) / fps
                f.write(f"DURATION: {duration:.5f}\n\n")

                for frame in range(frame_start, frame_end + 1):
                    bpy.context.scene.frame_set(frame)
                    timestamp = frame / fps
                    f.write(f"KEYFRAME: {frame}\n")
                    f.write(f"TIMESTAMP: {timestamp:.5f}\n")

                    for bone in armature.pose.bones:
                        parent_matrix = bone.parent.matrix if bone.parent else mathutils.Matrix.Identity(4)
                        local_matrix = parent_matrix.inverted_safe() @ bone.matrix
                        position = local_matrix.translation

                        rotation = local_matrix.to_quaternion()
                        qw = rotation.w
                        qx = rotation.x
                        qy = rotation.y
                        qz = rotation.z
                        scale = bone.scale

                        f.write(f"{position.x:.5f} {position.y:.5f} {position.z:.5f}\n")
                        f.write(f"{qx:.5f} {qy:.5f} {qz:.5f} {qw:.5f}\n")
                        f.write(f"{scale.x:.5f} {scale.y:.5f} {scale.z:.5f}\n\n")

        armature.animation_data.action = None  # Clear the assigned action to avoid conflicts

def export_mesh_with_indices(filepath):
    with open(filepath, "w") as f:
        meshes = [obj for obj in bpy.context.selected_objects if obj.type == 'MESH']
        if not meshes:
            print("No mesh selected for export.")
            return

        for mesh in meshes:
            mesh_data = mesh.data
            f.write(f"MESH_NAME: {mesh.name}\n")

            # Parse base colors from Principled BSDF nodes in materials
            material_colors = []
            if mesh_data.materials:
                for material in mesh_data.materials:
                    base_color = (1.0, 1.0, 1.0, 1.0)  # default fallback
                    if material and material.use_nodes:
                        for node in material.node_tree.nodes:
                            if node.type == "BSDF_PRINCIPLED":
                                base_color = node.inputs['Base Color'].default_value[:]
                                break
                    material_colors.append(base_color)

            unique_vertices = []
            vertex_map = {}

            mesh_eval = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
            mesh_eval_data = mesh_eval.to_mesh()
            conv = axis_conversion(from_forward='-Y', from_up='Z', to_forward='-Z', to_up='Y').to_4x4()
            mesh_eval_data.transform(conv)

            uv_layer = mesh_eval_data.uv_layers.active
            submesh_count = max(1, len(mesh_eval_data.materials))
            index_buffers = [[] for _ in range(submesh_count)]

            for poly in mesh_eval_data.polygons:
                mat_idx = poly.material_index
                color = material_colors[mat_idx] if mat_idx < len(material_colors) else (1.0, 1.0, 1.0, 1.0)

                for loop_index in poly.loop_indices:
                    loop = mesh_eval_data.loops[loop_index]
                    vert = mesh_eval_data.vertices[loop.vertex_index]

                    position = mesh.matrix_world @ vert.co
                    normal = mesh.matrix_world.to_3x3() @ vert.normal

                    if uv_layer:
                        uv = uv_layer.data[loop.index].uv
                        uv_tuple = (uv.x, uv.y)
                    else:
                        uv_tuple = (0.0, 0.0)

                    vertex_weights = []
                    for group in vert.groups:
                        group_index = group.group
                        weight = group.weight

                        if group_index < len(mesh.vertex_groups):
                            bone_name = mesh.vertex_groups[group_index].name
                            vertex_weights.append((bone_name, round(weight, 6)))

                    # Include material color in vertex key
                    vertex_key = (position.x, position.y, position.z,
                                  normal.x, normal.y, normal.z,
                                  uv_tuple, tuple(vertex_weights), tuple(color))

                    if vertex_key not in vertex_map:
                        vertex_map[vertex_key] = len(unique_vertices)
                        unique_vertices.append(vertex_key)

                    index_buffers[mat_idx].append(vertex_map[vertex_key])

            indices = []
            for buffer in index_buffers:
                indices.extend(buffer)

            f.write(f"VERTEX_COUNT: {len(unique_vertices)}\n")
            for v in unique_vertices:
                pos = v[0:3]
                norm = v[3:6]
                uv = v[6]
                weights = v[7]
                color = v[8]

                f.write(f"VERT:\n{pos[0]:.5f} {pos[1]:.5f} {pos[2]:.5f}\n")
                f.write(f"{norm[0]:.5f} {norm[1]:.5f} {norm[2]:.5f}\n")
                f.write(f"{uv[0]:.5f} {uv[1]:.5f}\n")
                # f.write(f"COLOR: {color[0]:.4f} {color[1]:.4f} {color[2]:.4f} {color[3]:.4f}\n")

                if weights:
                    text = " ".join(f"{bone} {weight}" for bone, weight in weights)
                    f.write(text + "\n\n")
                else:
                    f.write("WEIGHTS: None\n\n")

            f.write(f"INDEX_COUNT: {len(indices)}\n")
            for i in range(0, len(indices), 3):
                if i + 2 < len(indices):
                    f.write(f"{indices[i]} {indices[i+1]} {indices[i+2]} ")
            f.write("\n\n")

armature_output = os.path.expanduser("E:/Software_Dev/rust/rust-opengl-engine/resources/models/animated/002_y_robot/y_robot_idle_bones.txt")
mesh_output = os.path.expanduser("E:/Software_Dev/rust/rust-opengl-engine/resources/models/static/stump/001_stump.txt")

# export_animation_data(armature_output)

bpy.context.scene.frame_set(0)
export_mesh_with_indices(mesh_output)
