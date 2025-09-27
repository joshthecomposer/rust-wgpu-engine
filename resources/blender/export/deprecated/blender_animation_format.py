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

        if armature.animation_data and armature.animation_data.action:
            action = armature.animation_data.action

            
            f.write(f"# ========== {action.name} ==========\n")


            frame_start = int(1)
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
                    local_matrix  = parent_matrix.inverted_safe() @ bone.matrix
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

                    

def export_mesh_with_indices(filepath):
    with open(filepath, "w") as f:
        meshes = [obj for obj in bpy.context.selected_objects if obj.type == 'MESH']
        if not meshes:
            print("No mesh selected for export.")
            return
        
        for mesh in meshes:
            f.write(f"DIFFUSE: None\n")
            f.write(f"SPECULAR: None\n")
            f.write(f"EMISSIVE: None\n")
            # Ensure mesh is triangulated (prevents quads/n-gons causing errors)
            # TODO: This doesn't work sometimes and we just have to clean it up in blender UI.
            bpy.ops.object.mode_set(mode='OBJECT')  # Ensure in object mode
            bpy.ops.object.select_all(action='DESELECT')
            mesh.select_set(True)
            bpy.context.view_layer.objects.active = mesh
            bpy.ops.object.mode_set(mode='EDIT')
            bpy.ops.mesh.quads_convert_to_tris()  # Triangulates the mesh
            bpy.ops.object.mode_set(mode='OBJECT')
            mesh_data = mesh.data

            f.write(f"MESH_NAME: {mesh.name}\n")
            
            # Dictionary to store unique vertices
            unique_vertices = []
            vertex_map = {}  # Maps (position, normal, UV, weights) → index
            indices = []  # Index buffer
            
            # Ensure mesh is in world space
            mesh_eval = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
            mesh_eval_data = mesh_eval.to_mesh()

            uv_layer = mesh_data.uv_layers.active  # Get UV layer (if exists)

            for poly in mesh_eval_data.polygons:
                for loop_index in poly.loop_indices:
                    loop = mesh_eval_data.loops[loop_index]
                    vert = mesh_eval_data.vertices[loop.vertex_index]

                    # Get vertex position
                    position = mesh.matrix_world @ vert.co

                    # Get normal
                    normal = mesh.matrix_world.to_3x3() @ vert.normal

                    # Get UVs (if available)
                    if uv_layer:
                        uv = uv_layer.data[loop.index].uv
                        uv_tuple = (round(uv.x, 6), round(uv.y, 6))
                    else:
                        uv_tuple = (0.0, 0.0)

                    # Get bone weights (if any)
                    vertex_weights = []
                    for group in vert.groups:
                        group_index = group.group  # Vertex group index
                        weight = group.weight

                        if group_index < len(mesh.vertex_groups):
                            bone_name = mesh.vertex_groups[group_index].name
                            vertex_weights.append((bone_name, round(weight, 6)))

                    # Unique key for vertex
                    vertex_key = (position.x, position.y, position.z, normal.x, normal.y, normal.z, uv_tuple, tuple(vertex_weights))

                    if vertex_key not in vertex_map:
                        vertex_map[vertex_key] = len(unique_vertices)
                        unique_vertices.append(vertex_key)

                    # Append the index
                    indices.append(vertex_map[vertex_key])

            # Export unique vertices
            f.write(f"VERTEX_COUNT: {len(unique_vertices)}\n")
            for v in unique_vertices:
                pos = v[:3]
                norm = v[3:6]
                uv = v[6]
                weights = v[7]

                f.write(f"VERT:\n{-pos[0]:.5f} {pos[2]:.5f} {pos[1]:.5f}\n{-norm[0]:.5f} {norm[2]:.5f} {norm[1]:.5f}\n{uv[0]:.5f} {uv[1]:.5f}\n")

                if weights:
                    text = " ".join(f"{bone} {weight}" for bone, weight in weights)
                    f.write(text + "\n\n")
                else:
                    f.write("WEIGHTS: None\n\n")

            # Export indices
            f.write(f"INDEX_COUNT: {len(indices)}\n")
            for i in range(0, len(indices), 3):
                f.write(f"{indices[i]} {indices[i+1]} {indices[i+2]} ")

armature_output = os.path.expanduser("E:/Software_Dev/rust/rust-opengl-engine/resources/models/animated/001_moose/testing_textures/moose_bones.txt")
mesh_output = os.path.expanduser("E:/Software_Dev/rust/rust-opengl-engine/resources/models/animated/001_moose/testing_textures/moose_model.txt")


export_animation_data(armature_output)
#bpy.context.scene.frame_set(current_frame)

current_frame = bpy.context.scene.frame_current
bpy.context.scene.frame_set(0)
export_mesh_with_indices(mesh_output)
bpy.context.scene.frame_set(current_frame)
