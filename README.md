A 3D game engine built in Rust with OpenGL.

## Windows Setup

### Prerequisites

1. **Install MSVC Toolchain**
   - Download and install [Visual Studio Build Tools 2022](https://my.visualstudio.com/Downloads?q=Visual%20Studio%202022)
   - Select "Desktop development with C++" workload

### Running
Run `cargo run` from the root directory.

---
## Exporting Models with Blender
### Exporting Static Models (No animation)
1. All models need to have at-minimum a diffuse texture. Also we don't currently support PBR textures. If you don't want to deal with textures, just use a dummy texture file.
2. All models need to be facing +Y for the easiest integration. It is possible to do a local transform correction, but just align your model with +Y. With weapons this means the weapon should be standing straight up wth the sharp part of the blade facing +Y.
3. Triangulate all faces in blender, when the texture is exported we use the standard of trianguated vertices with a list of indices that map to vertices (for less overall data)  
   - Select your model  
   - switch to edit mode  
   - ctrl + T to triangulate

<img width="2552" height="1429" alt="image" src="https://github.com/user-attachments/assets/03462a13-44de-4dc0-9bd0-4ef2345129af" />  

3. For cleanliness, delete the camera and light sources (optional)  

4. Apply all transforms in blender. In object mode select your object and hit ctrl + A -> select "All transforms" If you don't apply your transforms, you're going to have a bad time.  

<img width="1126" height="712" alt="image" src="https://github.com/user-attachments/assets/368ccdac-c612-4bf6-a56b-67d8fb5e1e39" />  

5. In the script section, open up the `blender_mesh_only_export.py`  

<img width="2558" height="1427" alt="image" src="https://github.com/user-attachments/assets/0d50c575-6c91-4ff8-9424-d1b22e326238" />  

6. At the bottom of the script, fill in the mesh_output variable with the path you want it to go to, we use .txt files like chads.  

7. Also fill out the diffuse texture name.  

8. ???  

10. Profit  

### Exporting Animated Models  

1. TBD  
