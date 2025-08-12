import bpy
import os
import mathutils
import math
from bpy_extras.io_utils import axis_conversion
from mathutils import Color

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
    def get_material_rgba(mat):
        # Try Principled BSDF base color first
        if mat and mat.use_nodes and mat.node_tree:
            for n in mat.node_tree.nodes:
                if n.type == 'BSDF_PRINCIPLED':
                    col = n.inputs.get("Base Color")
                    if col and col.default_value:
                        v = col.default_value  # RGBA
                        return (float(v[0]), float(v[1]), float(v[2]), float(v[3]))
        # Fallback to diffuse_color
        if mat and getattr(mat, "diffuse_color", None):
            v = mat.diffuse_color  # RGBA
            return (float(v[0]), float(v[1]), float(v[2]), float(v[3] if len(v) > 3 else 1.0))
        return (1.0, 1.0, 1.0, 1.0)

    with open(filepath, "w") as f:
        meshes = [obj for obj in bpy.context.selected_objects if obj.type == 'MESH']
        if not meshes:
            print("No mesh selected for export.")
            return

        for mesh in meshes:
            mesh_data = mesh.data

            # Export materials and their flat colors
            if mesh_data.materials:
                for i, material in enumerate(mesh_data.materials):
                    f.write(f"TEXTURE_DIFFUSE: {material.name}\n")
                    mr, mg, mb, ma = get_material_rgba(material)
                    f.write(f"MATERIAL_COLOR {i}: {mr:.6f} {mg:.6f} {mb:.6f} {ma:.6f}\n")

            f.write(f"MESH_NAME: {mesh.name}\n")

            unique_vertices = []
            vertex_map = {}  # (pos, norm, uv, color, weights) -> idx

            # Evaluate mesh in world space + axis conversion
            depsgraph = bpy.context.evaluated_depsgraph_get()
            mesh_eval = mesh.evaluated_get(depsgraph)
            mesh_eval_data = mesh_eval.to_mesh()
            conv = axis_conversion(from_forward='-Y', from_up='Z', to_forward='-Z', to_up='Y').to_4x4()
            mesh_eval_data.transform(conv)

            uv_layer = mesh_eval_data.uv_layers.active
            # Active vertex color attribute (Blender 3.x API)
            col_attr = getattr(mesh_eval_data, "color_attributes", None)
            col_layer = col_attr.active if (col_attr and col_attr.active) else None

            submesh_count = max(1, len(mesh_eval_data.materials))
            index_buffers = [[] for _ in range(submesh_count)]

            for poly in mesh_eval_data.polygons:
                for loop_index in poly.loop_indices:
                    loop = mesh_eval_data.loops[loop_index]   # <-- use eval loops
                    vert = mesh_eval_data.vertices[loop.vertex_index]

                    # Position/normal in object space already transformed by conv; apply object transform too
                    position = (mesh.matrix_world @ vert.co)
                    normal = (mesh.matrix_world.to_3x3() @ vert.normal)

                    # UV (per corner)
                    if uv_layer:
                        uv = uv_layer.data[loop_index].uv
                        uv_tuple = (float(uv.x), float(uv.y))
                    else:
                        uv_tuple = (0.0, 0.0)

                    # Color (per corner if vertex colors exist; else fallback to material color)
                    if col_layer and col_layer.domain == 'CORNER':
                        c = col_layer.data[loop_index].color
                        # Some Blender versions store RGB only; ensure RGBA
                        if len(c) >= 4:
                            col_tuple = (float(c[0]), float(c[1]), float(c[2]), float(c[3]))
                        else:
                            col_tuple = (float(c[0]), float(c[1]), float(c[2]), 1.0)
                    else:
                        mat = mesh_data.materials[poly.material_index] if mesh_data.materials else None
                        col_tuple = get_material_rgba(mat)

                    # Bone weights (by vertex)
                    vertex_weights = []
                    for g in vert.groups:
                        gi = g.group
                        w = g.weight
                        if gi < len(mesh.vertex_groups):
                            bone_name = mesh.vertex_groups[gi].name
                            vertex_weights.append((bone_name, round(w, 6)))

                    vertex_key = (
                        float(position.x), float(position.y), float(position.z),
                        float(normal.x), float(normal.y), float(normal.z),
                        uv_tuple,
                        col_tuple,                 # include color in uniqueness!
                        tuple(vertex_weights)
                    )

                    if vertex_key not in vertex_map:
                        vertex_map[vertex_key] = len(unique_vertices)
                        unique_vertices.append(vertex_key)

                    material_index = poly.material_index
                    index_buffers[material_index].append(vertex_map[vertex_key])

            # Flatten indices
            indices = []
            for buffer in index_buffers:
                indices.extend(buffer)

            # Write vertices (with color)
            f.write(f"VERTEX_COUNT: {len(unique_vertices)}\n")
            for v in unique_vertices:
                pos = v[:3]
                norm = v[3:6]
                uv = v[6]
                col = v[7]
                weights = v[8]

                f.write(
                    "VERT:\n"
                    f"{pos[0]:.5f} {pos[1]:.5f} {pos[2]:.5f}\n"
                    f"{norm[0]:.5f} {norm[1]:.5f} {norm[2]:.5f}\n"
                    f"{uv[0]:.5f} {(1.0 - uv[1]):.5f}\n"
                    f"COLOR: {col[0]:.5f} {col[1]:.5f} {col[2]:.5f} {col[3]:.5f}\n"
                )

                if weights:
                    text = " ".join(f"{bone} {weight}" for bone, weight in weights)
                    f.write(text + "\n\n")
                else:
                    f.write("WEIGHTS: None\n\n")

            # Write indices (triangles)
            f.write(f"INDEX_COUNT: {len(indices)}\n")
            for i in range(0, len(indices), 3):
                if i + 2 < len(indices):
                    f.write(f"{indices[i]} {indices[i+1]} {indices[i+2]} ")

armature_output = os.path.expanduser("E:/Software_Dev/rust/rust-opengl-engine/resources/models/animated/002_y_robot/y_robot_base_color_bones.txt")
mesh_output = os.path.expanduser("E:/Software_Dev/rust/rust-opengl-engine/resources/models/animated/002_y_robot/y_robot_base_color_mesh.txt")


export_animation_data(armature_output)
#bpy.context.scene.frame_set(current_frame)

current_frame = bpy.context.scene.frame_current
bpy.context.scene.frame_set(0)
export_mesh_with_indices(mesh_output)
bpy.context.scene.frame_set(current_frame)
