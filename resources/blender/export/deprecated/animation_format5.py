import bpy
import os
import mathutils
import math
from bpy_extras.io_utils import axis_conversion
import shutil

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

def export_mesh_with_indices(filepath, diffuse_texture):
    with open(filepath, "w") as f:
        meshes = [obj for obj in bpy.context.selected_objects if obj.type == 'MESH']
        if not meshes:
            print("No mesh selected for export.")
            return

        for mesh in meshes:
            mesh_data = mesh.data
            f.write(f"MESH_NAME: {mesh.name}\n")
            f.write(f"TEXTURE_DIFFUSE: {diffuse_texture}\n")

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
            
            Nmat = mesh.matrix_world.to_3x3().inverted().transposed()

            for poly in mesh_eval_data.polygons:
                mat_idx = poly.material_index
                color = material_colors[mat_idx] if mat_idx < len(material_colors) else (1.0, 1.0, 1.0, 1.0)

                for loop_index in poly.loop_indices:
                    loop = mesh_eval_data.loops[loop_index]
                    vert = mesh_eval_data.vertices[loop.vertex_index]

                    position = mesh.matrix_world @ vert.co
                    #normal = mesh.matrix_world.to_3x3() @ vert.normal
                    loop_normal_local = loop.normal
                    normal_world = (Nmat @ loop_normal_local).normalized()

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
                                  normal_world.x, normal_world.y, normal_world.z,
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
                f.write(f"{uv[0]:.5f} {1.0 - uv[1]:.5f}\n")
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
            
def move_texture_file(diffuse_texture_path, parent_dir):
    shutil.copy(diffuse_texture_path, parent_dir)

diffuse_texture_path = Path(r"E:\Software_Dev\rust\rust-opengl-engine\resources\models\static\terrain\rocks\stonetex.png")
mesh_output_path = Path(r"C:\Users\jdwis\OneDrive\Desktop\Output\tube.txt")
output_parent_dir = mesh_output_path.parent
os.makedirs(output_parent_dir, exist_ok=True)
diffuse_texture = "stonetex.png"
diffuse_texture_path = Path(r"C:\Users\jdwis\OneDrive\Desktop\Output\sphere_rock.txt")
move_texture_file(diffuse_texture_path, output_parent_dir)

export_mesh_with_indices(mesh_output_path, diffuse_texture)