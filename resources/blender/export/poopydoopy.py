import bpy
import mathutils
from bpy_extras.io_utils import axis_conversion

# ---------- Helpers ----------
C_LEGACY = mathutils.Matrix((
    (1,  0,  0,  0),
    (0,  0,  1,  0),
    (0, -1,  0,  0),
    (0,  0,  0,  1)
))

def conv_mats():
    # Z-up, -Y fwd  ->  Y-up, -Z fwd  (Blender -> your OpenGL convention)
    C = axis_conversion(from_forward='-Y', from_up='Z', to_forward='-Z', to_up='Y').to_4x4()
    C3 = C.to_3x3()
    # For normals
    C3_invT = C3.inverted().transposed()
    return C, C3, C3_invT, C.to_quaternion()

def world_mats(obj):
    W = obj.matrix_world.copy()
    W3 = W.to_3x3()
    W3_invT = W3.inverted().transposed()
    return W, W3, W3_invT

def inv_bind_matrix(C, armW, bone_rest_local):
    # Full rest in world: ArmW * BoneRestLocal
    Rw = armW @ bone_rest_local
    # Convert basis on the left: C @ Rw
    return (C @ Rw).inverted()

def convert_TRS_local(C3, Qc, T, Q, S):
    Tp = C3 @ T
    Qp = Qc @ Q @ Qc.inverted()
    # S is fine in most axis changes (uniform/non-shearing)
    return Tp, Qp, S

def invT_linear(M4):
    # inverse-transpose of linear 3x3 part of a 4x4
    M3 = M4.to_3x3()
    return M3.inverted().transposed()

def write_mat_like_old(f, M: mathutils.Matrix):
    Mt = M.transposed()               # <-- transpose for legacy row-major on disk
    for r in Mt:
        f.write(f"{r[0]:.5f} {r[1]:.5f} {r[2]:.5f} {r[3]:.5f}\n")
        
def get_material_rgba(mat):
    if mat and getattr(mat, "use_nodes", False) and mat.node_tree:
        for n in mat.node_tree.nodes:
            if n.type == 'BSDF_PRINCIPLED':
                col = n.inputs.get("Base Color")
                if col and col.default_value:
                    v = col.default_value
                    return (float(v[0]), float(v[1]), float(v[2]), float(v[3]))
    if mat and getattr(mat, "diffuse_color", None):
        v = mat.diffuse_color
        return (float(v[0]), float(v[1]), float(v[2]), float(v[3] if len(v) > 3 else 1.0))
    return (1.0, 1.0, 1.0, 1.0)

# ---------- Skeleton + Anim export (fixed) ----------

# =========================
# Skeleton + Anim Export
# =========================
def write_skeleton_and_anims(f, armature, mesh_obj):
    C = axis_conversion(from_forward='-Y', from_up='Z', to_forward='-Z', to_up='Y').to_4x4()
    C_inv = C.inverted()

    pose_bones = armature.pose.bones
    bones = list(pose_bones)                       # <-- use POSE order
    name_to_index = {b.name: i for i, b in enumerate(bones)}

    f.write("\nSKELETON_DATA ####################################\n\n")
    f.write(f"BONECOUNT: {len(bones)}\n")

    # Match old: global = armature.matrix_world^-1 (old code transposed the matrix; we transpose on write)
    G = armature.matrix_world.copy().inverted()    # <-- NO .transposed() here
    f.write("GLOBAL_TRANSFORM:\n")
    write_mat_like_old(f, G)                       # <-- this transposes on write
    f.write("\n")

    # Temporarily convert armature DATA like before (we'll undo later)
    armature.data.transform(C)
    try:
        for pbone in bones:
            parent_index = -1 if pbone.parent is None else name_to_index[pbone.parent.name]
            f.write(f"BONE_NAME: {pbone.name}\nPARENT_INDEX: {parent_index}\n")
            f.write("OFFSET_MATRIX:\n")

            # Old exporter did: bone.bone.matrix_local.inverted().transposed()
            # We avoid double transpose by leaving it untransposed here:
            Off = pbone.bone.matrix_local.inverted()   # <-- NO .transposed() here
            write_mat_like_old(f, Off)                 # <-- transposes on write
            f.write("\n")

        # --- Animations (identical logic to working exporter) ---
        f.write("\nANIMATION_DATA ####################################\n\n")
        fps = bpy.context.scene.render.fps
        f.write(f"FPS: {fps}\n")

        current_frame = bpy.context.scene.frame_current
        try:
            for action in bpy.data.actions:
                armature.animation_data.action = action
                if not armature.animation_data or not armature.animation_data.action:
                    continue

                fs = int(action.frame_range[0]); fe = int(action.frame_range[1])
                duration = (fe - fs) / fps
                f.write(f"ANIMATION_NAME: {action.name}\n")
                f.write(f"DURATION: {duration:.5f}\n\n")

                for frame in range(fs, fe + 1):
                    bpy.context.scene.frame_set(frame)
                    t = frame / fps
                    f.write(f"KEYFRAME: {frame}\n")
                    f.write(f"TIMESTAMP: {t:.5f}\n")

                    for pb in bones:  # <-- SAME ORDER as skeleton
                        pbone = pose_bones[pb.name]
                        parent_matrix = pbone.parent.matrix if pbone.parent else mathutils.Matrix.Identity(4)
                        local_matrix = parent_matrix.inverted_safe() @ pbone.matrix

                        pos = local_matrix.translation
                        rot = local_matrix.to_quaternion()
                        scl = local_matrix.to_scale()

                        f.write(f"{pos.x:.5f} {pos.y:.5f} {pos.z:.5f}\n")
                        f.write(f"{rot.x:.5f} {rot.y:.5f} {rot.z:.5f} {rot.w:.5f}\n")
                        f.write(f"{scl.x:.5f} {scl.y:.5f} {scl.z:.5f}\n\n")
                f.write("\n")
        finally:
            bpy.context.scene.frame_set(current_frame)
            armature.animation_data.action = None

    finally:
        # Undo the temporary conversion so the .blend is unchanged
        armature.data.transform(C_inv)

    return name_to_index

# ---------- Mesh export (fixed normals + winding + weights) ----------

# =========================
# Mesh Export
# =========================
def write_mesh(f, mesh_obj, bone_index_of):
    C = axis_conversion(from_forward='-Y', from_up='Z', to_forward='-Z', to_up='Y').to_4x4()

    deps = bpy.context.evaluated_depsgraph_get()
    mesh_eval = mesh_obj.evaluated_get(deps)
    me_eval = mesh_eval.to_mesh()
    try:
        # Match working exporter: pre-apply conversion to the evaluated mesh
        me_eval.transform(C)

        uv_layer = me_eval.uv_layers.active
        col_attr = getattr(me_eval, "color_attributes", None)
        col_layer = col_attr.active if (col_attr and col_attr.active) else None

        unique, vmap, indices = [], {}, []

        # If you want to split by materials, you can track poly.material_index
        for poly in me_eval.polygons:
            face_idx = []
            for li in poly.loop_indices:
                loop = me_eval.loops[li]
                v = me_eval.vertices[loop.vertex_index]

                # Positions/normals in world space (like the working script)
                p = mesh_obj.matrix_world @ v.co
                n = (mesh_obj.matrix_world.to_3x3() @ v.normal).normalized()

                if uv_layer:
                    uv = uv_layer.data[li].uv
                    uv_tuple = (float(uv.x), float(1.0 - uv.y))
                else:
                    uv_tuple = (0.0, 0.0)

                if col_layer and col_layer.domain == 'CORNER':
                    c = col_layer.data[li].color
                    col = (float(c[0]), float(c[1]), float(c[2]), float(c[3] if len(c) > 3 else 1.0))
                else:
                    src_mat = me_eval.materials[poly.material_index] if me_eval.materials else None
                    col = get_material_rgba(src_mat)

                # Bone weights as NAME+weight pairs (top 4, pad)
                weights = []
                for g in v.groups:
                    if g.group < len(mesh_obj.vertex_groups):
                        bone_name = mesh_obj.vertex_groups[g.group].name
                        weights.append((bone_name, round(g.weight, 6)))
                weights.sort(key=lambda x: x[1], reverse=True)
                weights = (weights + [("", 0.0)] * 4)[:4]

                key = (
                    round(p.x,6), round(p.y,6), round(p.z,6),
                    round(n.x,6), round(n.y,6), round(n.z,6),
                    round(uv_tuple[0],6), round(uv_tuple[1],6),
                    tuple(weights), tuple(col)
                )
                if key not in vmap:
                    vmap[key] = len(unique)
                    unique.append((p, n, uv_tuple, col, weights))
                face_idx.append(vmap[key])

            for i in range(1, len(face_idx)-1):
                indices.extend([face_idx[0], face_idx[i], face_idx[i+1]])

        f.write("\nMESH_DATA ##############################\n")
        f.write(f"MESH_NAME: {mesh_obj.name}\n")
        f.write(f"VERTEX_COUNT: {len(unique)}\n")
        for (p, n, uv, col, weights) in unique:
            f.write("VERT:\n")
            f.write(f"{p.x:.5f} {p.y:.5f} {p.z:.5f}\n")
            f.write(f"{n.x:.5f} {n.y:.5f} {n.z:.5f}\n")
            f.write(f"{uv[0]:.5f} {uv[1]:.5f}\n")
            f.write(f"COLOR: {col[0]:.5f} {col[1]:.5f} {col[2]:.5f} {col[3]:.5f}\n")

            if any(name and w > 0 for name, w in weights):
                f.write(" ".join(f"{name} {w:.5f}" for name, w in weights if name and w > 0) + "\n")
            else:
                f.write("None\n")
            f.write("\n")

        f.write(f"INDEX_COUNT: {len(indices)}\n")
        for i in range(0, len(indices), 3):
            if i+2 < len(indices):
                f.write(f"{indices[i]} {indices[i+1]} {indices[i+2]} ")
        f.write("\n")
    finally:
        # Free evaluated mesh
        mesh_eval.to_mesh_clear()

def export_game_data(filepath):
    _, _, _, _ = conv_mats()
    cur = bpy.context.scene.frame_current
    try:
        with open(filepath, "w") as f:
            f.write("# WiseModel 0.0.1\n")

            armatures = [o for o in bpy.context.selected_objects if o.type == 'ARMATURE']
            meshes    = [o for o in bpy.context.selected_objects if o.type == 'MESH']
            if not armatures or not meshes:
                print("Select one armature and one mesh.")
                return {'CANCELLED'}

            arm  = armatures[0]
            mesh = meshes[0]

            bone_index_of = write_skeleton_and_anims(f, arm, mesh)

            # ✅ freeze pose for mesh export
            bpy.context.scene.frame_set(0)
            write_mesh(f, mesh, bone_index_of)

        print(f"Exported data to {filepath}")
    finally:
        bpy.context.scene.frame_set(cur)
             
export_game_data("E:/Software_Dev/rust/rust-opengl-engine/resources/models/animated/roby.txt")