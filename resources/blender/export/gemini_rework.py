import bpy
import os
import mathutils
from bpy_extras.io_utils import axis_conversion

# A Blender script intended to be run as an Add-on or directly in the text editor.
# It exports selected mesh and armature data into a custom text file format
# for use in an OpenGL game engine.

def write_matrix_to_file(file, matrix):
    """Helper function to write a 4x4 matrix to a file."""
    for row in matrix:
        file.write(f"{row[0]:.5f} {row[1]:.5f} {row[2]:.5f} {row[3]:.5f}\n")

def export_game_data(filepath):
    """
    Main function to export mesh, skeleton, and animation data.
    This function processes the selected objects and writes all relevant data
    to a single text file.
    """
    # Define the coordinate system conversion matrix
    # Blender's default is Z-up, Y-forward. OpenGL is typically Y-up, Z-backward.
    # This matrix transforms vectors from Blender to OpenGL space.
    # Note: We apply this transformation to the final data, not the source mesh.
    conv_matrix = axis_conversion(from_forward='-Y', from_up='Z', to_forward='-Z', to_up='Y').to_4x4()
    
    # Store the current frame so we can restore it later
    current_frame = bpy.context.scene.frame_current

    try:
        with open(filepath, "w") as f:
            f.write("# WiseModel 0.0.1\n")

            # --- Find and validate selected objects ---
            armatures = [obj for obj in bpy.context.selected_objects if obj.type == 'ARMATURE']
            meshes = [obj for obj in bpy.context.selected_objects if obj.type == 'MESH']

            if not armatures:
                print("No armature selected for export.")
                return {'CANCELLED'}
            if not meshes:
                print("No mesh selected for export.")
                return {'CANCELLED'}

            armature = armatures[0]
            mesh = meshes[0]
            
            # --- Export Skeleton and Bind Pose Data ---
            f.write("\nSKELETON_DATA ####################################\n\n")
            f.write(f"BONECOUNT: {len(armature.pose.bones)}\n")
            
            # Write global inverse transform of the armature
            global_transform = conv_matrix @ armature.matrix_world.inverted()
            f.write("GLOBAL_TRANSFORM:\n")
            write_matrix_to_file(f, global_transform)
            f.write("\n")
            
            # Write each bone's bind pose information
            for bone in armature.pose.bones:
                parent_index = -1
                if bone.parent:
                    try:
                        parent_index = list(armature.pose.bones).index(bone.parent)
                    except ValueError:
                        print(f"Warning: Parent bone '{bone.parent.name}' not found in selected armature.")

                f.write(f"BONE_NAME: {bone.name}\nPARENT_INDEX: {parent_index}\n")
                
                # Calculate and write the inverse bind pose matrix
                # Note: We apply the coordinate system conversion here
                offset_matrix = conv_matrix @ armature.matrix_world @ bone.bone.matrix_local.inverted()
                f.write("OFFSET_MATRIX:\n")
                write_matrix_to_file(f, offset_matrix)
                f.write("\n")
            f.write("\n")

            # --- Export Animation Data ---
            f.write("\nANIMATION_DATA ####################################\n\n")
            fps = bpy.context.scene.render.fps
            f.write(f"FPS: {fps}\n")
            
            for action in bpy.data.actions:
                # Temporarily assign action to armature
                armature.animation_data.action = action
                if not armature.animation_data or not armature.animation_data.action:
                    continue

                frame_start = int(action.frame_range[0])
                frame_end = int(action.frame_range[1])
                duration = (frame_end - frame_start) / fps
                
                f.write(f"ANIMATION_NAME: {action.name}\n")
                f.write(f"DURATION: {duration:.5f}\n")
                
                # Iterate through keyframes and export bone transforms
                for frame in range(frame_start, frame_end + 1):
                    bpy.context.scene.frame_set(frame)
                    
                    timestamp = frame / fps
                    f.write(f"KEYFRAME: {frame}\n")
                    f.write(f"TIMESTAMP: {timestamp:.5f}\n")

                    for bone in armature.pose.bones:
                        # Get the local transform of the bone
                        local_matrix = bone.matrix_basis
                        
                        # Decompose the local matrix
                        position = local_matrix.translation
                        rotation = local_matrix.to_quaternion()
                        scale = local_matrix.to_scale()
                        
                        f.write(f"{position.x:.5f} {position.y:.5f} {position.z:.5f}\n")
                        f.write(f"{rotation.x:.5f} {rotation.y:.5f} {rotation.z:.5f} {rotation.w:.5f}\n")
                        f.write(f"{scale.x:.5f} {scale.y:.5f} {scale.z:.5f}\n")
                        f.write("\n")
                
                f.write("\n")
            
            armature.animation_data.action = None

            # --- Export Mesh and Vertex Data ---
            f.write("\nMESH_DATA ##############################\n")
            f.write(f"MESH_NAME: {mesh.name}\n")
            
            # Use an evaluated mesh for correct modifiers
            depsgraph = bpy.context.evaluated_depsgraph_get()
            mesh_eval = mesh.evaluated_get(depsgraph)
            mesh_data = mesh_eval.to_mesh()
            
            # Get active UV and Vertex Color layers
            uv_layer = mesh_data.uv_layers.active
            col_attr = getattr(mesh_data, "color_attributes", None)
            col_layer = col_attr.active if (col_attr and col_attr.active) else None
            
            unique_vertices = []
            vertex_map = {}
            index_buffer = []

            for poly in mesh_data.polygons:
                for loop_index in poly.loop_indices:
                    loop = mesh_data.loops[loop_index]
                    vert = mesh_data.vertices[loop.vertex_index]
                    
                    # Position, Normal, UVs, Color
                    position = conv_matrix @ mesh.matrix_world @ vert.co
                    normal = conv_matrix.to_3x3() @ mesh.matrix_world.to_3x3() @ vert.normal
                    uv = uv_layer.data[loop_index].uv if uv_layer else mathutils.Vector((0.0, 0.0))
                    
                    col_tuple = (1.0, 1.0, 1.0, 1.0)
                    if col_layer and col_layer.domain == 'CORNER':
                        c = col_layer.data[loop_index].color
                        col_tuple = (float(c[0]), float(c[1]), float(c[2]), float(c[3]) if len(c) > 3 else 1.0)
                        
                    # Bone weights and indices
                    vertex_weights = []
                    for g in vert.groups:
                        bone_name = mesh.vertex_groups[g.group].name
                        weight = round(g.weight, 6)
                        vertex_weights.append((bone_name, weight))
                    
                    # Sort and pad weights to 4 bones per vertex
                    vertex_weights.sort(key=lambda x: x[1], reverse=True)
                    vertex_weights = vertex_weights[:4]
                    while len(vertex_weights) < 4:
                        vertex_weights.append(("", 0.0))
                    
                    # Create a unique key for each vertex
                    vertex_key = (
                        tuple(position), tuple(normal), tuple(uv), col_tuple, tuple(vertex_weights)
                    )
                    
                    # If this is a new vertex, add it to the unique list
                    if vertex_key not in vertex_map:
                        vertex_map[vertex_key] = len(unique_vertices)
                        unique_vertices.append(vertex_key)
                    
                    # Add the index to the index buffer
                    index_buffer.append(vertex_map[vertex_key])
            
            # Write unique vertices
            f.write(f"VERTEX_COUNT: {len(unique_vertices)}\n")
            for v in unique_vertices:
                pos, norm, uv, col, weights = v
                f.write("VERT:\n")
                f.write(f"{pos[0]:.5f} {pos[1]:.5f} {pos[2]:.5f}\n")
                f.write(f"{norm[0]:.5f} {norm[1]:.5f} {norm[2]:.5f}\n")
                f.write(f"{uv[0]:.5f} {(1.0 - uv[1]):.5f}\n")
                #f.write(f"{col[0]:.5f} {col[1]:.5f} {col[2]:.5f} {col[3]:.5f}\n")
                
                weight_string = " ".join(f"{bone} {weight:.5f}" for bone, weight in weights if bone)
                if weight_string:
                    f.write(f"{weight_string}\n")
                else:
                    f.write("None\n")
                f.write("\n")
            f.write("\n")

            # Write indices
            f.write(f"INDEX_COUNT: {len(index_buffer)}\n")
            for i in range(0, len(index_buffer), 3):
                if i + 2 < len(index_buffer):
                    f.write(f"{index_buffer[i]} {index_buffer[i+1]} {index_buffer[i+2]} ")
            f.write("\n")
            
            print(f"Exported data to {filepath}")
            
    finally:
        # Restore the original frame
        bpy.context.scene.frame_set(current_frame)
    
# Example usage to be called from the Blender text editor:
# Change the path below to your desired output file.
if __name__ == '__main__':
    # This example assumes you have an armature and mesh selected.
    # In a real add-on, you'd have a file dialog.
    filepath = os.path.expanduser("E:/Software_Dev/rust/rust-opengl-engine/resources/models/animated/exported_model.txt")
    export_game_data(filepath)
