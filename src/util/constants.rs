#![allow(dead_code)]
use glam::{vec3, Vec3};

#[rustfmt::skip]
pub const UNIT_CUBE_VERTICES: [f32; 6 * 6 * 8] = [
    // positions            // tex coords       // normals
    // Front face
    -0.5, -0.5, -0.5,       0.0, 0.0,           0.0,  0.0, -1.0,
    -0.5,  0.5, -0.5,       0.0, 1.0,           0.0,  0.0, -1.0,
     0.5,  0.5, -0.5,       1.0, 1.0,           0.0,  0.0, -1.0,
     0.5,  0.5, -0.5,       1.0, 1.0,           0.0,  0.0, -1.0,
     0.5, -0.5, -0.5,       1.0, 0.0,           0.0,  0.0, -1.0,
    -0.5, -0.5, -0.5,       0.0, 0.0,           0.0,  0.0, -1.0,

    // Back face
    -0.5, -0.5,  0.5,       0.0, 0.0,           0.0,  0.0,  1.0,
     0.5, -0.5,  0.5,       1.0, 0.0,           0.0,  0.0,  1.0,
     0.5,  0.5,  0.5,       1.0, 1.0,           0.0,  0.0,  1.0,
     0.5,  0.5,  0.5,       1.0, 1.0,           0.0,  0.0,  1.0,
    -0.5,  0.5,  0.5,       0.0, 1.0,           0.0,  0.0,  1.0,
    -0.5, -0.5,  0.5,       0.0, 0.0,           0.0,  0.0,  1.0,

    // Left face
    -0.5, -0.5, -0.5,       0.0, 0.0,          -1.0,  0.0,  0.0,
    -0.5, -0.5,  0.5,       1.0, 0.0,          -1.0,  0.0,  0.0,
    -0.5,  0.5,  0.5,       1.0, 1.0,          -1.0,  0.0,  0.0,
    -0.5,  0.5,  0.5,       1.0, 1.0,          -1.0,  0.0,  0.0,
    -0.5,  0.5, -0.5,       0.0, 1.0,          -1.0,  0.0,  0.0,
    -0.5, -0.5, -0.5,       0.0, 0.0,          -1.0,  0.0,  0.0,

    // Right face
     0.5, -0.5, -0.5,       0.0, 0.0,           1.0,  0.0,  0.0,
     0.5,  0.5, -0.5,       1.0, 0.0,           1.0,  0.0,  0.0,
     0.5,  0.5,  0.5,       1.0, 1.0,           1.0,  0.0,  0.0,
     0.5,  0.5,  0.5,       1.0, 1.0,           1.0,  0.0,  0.0,
     0.5, -0.5,  0.5,       0.0, 1.0,           1.0,  0.0,  0.0,
     0.5, -0.5, -0.5,       0.0, 0.0,           1.0,  0.0,  0.0,

    // Bottom face
    -0.5, -0.5, -0.5,       0.0, 0.0,           0.0, -1.0,  0.0,
     0.5, -0.5, -0.5,       1.0, 0.0,           0.0, -1.0,  0.0,
     0.5, -0.5,  0.5,       1.0, 1.0,           0.0, -1.0,  0.0,
     0.5, -0.5,  0.5,       1.0, 1.0,           0.0, -1.0,  0.0,
    -0.5, -0.5,  0.5,       0.0, 1.0,           0.0, -1.0,  0.0,
    -0.5, -0.5, -0.5,       0.0, 0.0,           0.0, -1.0,  0.0,

    // Top face
    -0.5,  0.5, -0.5,       0.0, 0.0,           0.0,  1.0,  0.0,
    -0.5,  0.5,  0.5,       0.0, 1.0,           0.0,  1.0,  0.0,
     0.5,  0.5,  0.5,       1.0, 1.0,           0.0,  1.0,  0.0,
     0.5,  0.5,  0.5,       1.0, 1.0,           0.0,  1.0,  0.0,
     0.5,  0.5, -0.5,       1.0, 0.0,           0.0,  1.0,  0.0,
    -0.5,  0.5, -0.5,       0.0, 0.0,           0.0,  1.0,  0.0,
];


#[rustfmt::skip]
pub const BASIC_QUAD_VERTICES: [f32; 30] = [
    // Positions       // Texture Coords
    -1.0,  1.0, 0.0,   0.0, 1.0,
    -1.0, -1.0, 0.0,   0.0, 0.0,
     1.0, -1.0, 0.0,   1.0, 0.0,

    -1.0,  1.0, 0.0,   0.0, 1.0,
     1.0, -1.0, 0.0,   1.0, 0.0,
     1.0,  1.0, 0.0,   1.0, 1.0,
];

pub const BISEXUAL_PINK:Vec3 = vec3(0.7294118, 0.11372549, 0.43529412);
pub const BISEXUAL_PINK_SCALE: Vec3 = vec3(
    1.0,
    0.15591398,
    0.59677416,
);
pub const BISEXUAL_PURPLE:Vec3 = vec3(0.54901961, 0.31764706, 0.58431373);
pub const BISEXUAL_PURPLE_SCALE: Vec3 = vec3(
    0.9395973,
    0.54362416,
    1.0,
);
pub const BISEXUAL_BLUE:Vec3 = vec3(0.19215686, 0.26666667, 0.6);
pub const BISEXUAL_BLUE_SCALE: Vec3 = vec3(
    0.32026142,
    0.44444445,
    1.0,
);
pub const CYBERPUNK_ORANGE:Vec3 = vec3(1.0,0.27058823,0.0);
pub const WHITE:Vec3 = vec3(1.0, 1.0, 1.0);

pub const CUBE_POSITIONS: [Vec3; 10] = [
    Vec3::new( 0.0,  0.0 + 5.0,  0.0), 
    Vec3::new( 2.0,  5.0 + 5.0, -15.0), 
    Vec3::new(-1.5, -2.2 + 5.0, -2.5),  
    Vec3::new(-3.8, -2.0 + 5.0, -12.3),  
    Vec3::new( 2.4, -0.4 + 5.0, -3.5),  
    Vec3::new(-1.7,  3.0 + 5.0, -7.5),  
    Vec3::new( 1.3, -2.0 + 5.0, -2.5),  
    Vec3::new( 1.5,  2.0 + 5.0, -2.5), 
    Vec3::new( 1.5,  0.2 + 5.0, -1.5), 
    Vec3::new(-1.3,  1.0 + 5.0, -1.5),  
];

//pub const FACES_CUBEMAP:[&str; 6] = [
//    "resources/textures/sky_box/Cartoon Base Skywave_Cam_2_Left+X.png",
//    "resources/textures/sky_box/Cartoon Base Skywave_Cam_3_Right-X.png",
//    "resources/textures/sky_box/Cartoon Base Skywave_Cam_4_Up+Y.png",
//    "resources/textures/sky_box/Cartoon Base Skywave_Cam_5_Down-Y.png",
//    "resources/textures/sky_box/Cartoon Base Skywave_Cam_0_Front+Z.png",
//    "resources/textures/sky_box/Cartoon Base Skywave_Cam_1_Back-Z.png",
//];

//pub const FACES_CUBEMAP:[&str; 6] = [
//    "resources/textures/sky_box2/CosmicCoolCloudLeft.png",
//    "resources/textures/sky_box2/CosmicCoolCloudRight.png",
//    "resources/textures/sky_box2/CosmicCoolCloudTop.png",
//    "resources/textures/sky_box2/CosmicCoolCloudBottom.png",
//    "resources/textures/sky_box2/CosmicCoolCloudFront.png",
//    "resources/textures/sky_box2/CosmicCoolCloudBack.png",
//];

// pub const FACES_CUBEMAP:[&str; 6] = [
//     "resources/textures/sky_box/AllSky_Space_Neon_Starless_Cam_2_Left+X.png",
//     "resources/textures/sky_box/AllSky_Space_Neon_Starless_Cam_3_Right-X.png",
//     "resources/textures/sky_box/AllSky_Space_Neon_Starless_Cam_4_Up+Y.png",
//     "resources/textures/sky_box/AllSky_Space_Neon_Starless_Cam_5_Down-Y.png",
//     "resources/textures/sky_box/AllSky_Space_Neon_Starless_Cam_0_Front+Z.png",
//     "resources/textures/sky_box/AllSky_Space_Neon_Starless_Cam_1_Back-Z.png",
// ];

//pub const FACES_CUBEMAP:[&str; 6] = [
//    "resources/textures/sky_box/Anime Night_Cam_2_Left+X.png",
//    "resources/textures/sky_box/Anime Night_Cam_3_Right-X.png",
//    "resources/textures/sky_box/Anime Night_Cam_4_Up+Y.png",
//    "resources/textures/sky_box/Anime Night_Cam_5_Down-Y.png",
//    "resources/textures/sky_box/Anime Night_Cam_0_Front+Z.png",
//    "resources/textures/sky_box/Anime Night_Cam_1_Back-Z.png",
//];

//pub const FACES_CUBEMAP:[&str; 6] = [
//    "resources/textures/sky_box/Sky_Day Sun High ClearHazy_Cam_1_Right+X.png",
//    "resources/textures/sky_box/Sky_Day Sun High ClearHazy_Cam_3_Left-X.png",
//    "resources/textures/sky_box/Sky_Day Sun High ClearHazy_Cam_4_Top+Y.png",
//    "resources/textures/sky_box/Sky_Day Sun High ClearHazy_Cam_5_Down-Y.png",
//    "resources/textures/sky_box/Sky_Day Sun High ClearHazy_Cam_0_Front+Z.png",
//    "resources/textures/sky_box/Sky_Day Sun High ClearHazy_Cam_2_Back-Z.png"
//];

pub const FACES_CUBEMAP: [&str; 6] = [
"resources/textures/sky_box/Sky_Space_Nebula_DeepBlack_Cam_2_Left+X.png",
"resources/textures/sky_box/Sky_Space_Nebula_DeepBlack_Cam_3_Right-X.png",
"resources/textures/sky_box/Sky_Space_Nebula_DeepBlack_Cam_4_Up+Y.png",
"resources/textures/sky_box/Sky_Space_Nebula_DeepBlack_Cam_5_Down-Y.png",
"resources/textures/sky_box/Sky_Space_Nebula_DeepBlack_Cam_0_Front+Z.png",
"resources/textures/sky_box/Sky_Space_Nebula_DeepBlack_Cam_1_Back-Z.png",
];



#[rustfmt::skip]
pub const SKYBOX_VERTICES:[f32; 24] = [
		-1.0, -1.0,  1.0,
		 1.0, -1.0,  1.0,
		 1.0, -1.0, -1.0,
		-1.0, -1.0, -1.0,
		-1.0,  1.0,  1.0,
		 1.0,  1.0,  1.0,
		 1.0,  1.0, -1.0,
		-1.0,  1.0, -1.0
];

#[rustfmt::skip]
pub const SKYBOX_INDICES:[u32; 36] = [
		// Right
		1, 2, 6,
		6, 5, 1,
		// Left
		0, 4, 7,
		7, 3, 0,
		// Top
		4, 5, 6,
		6, 7, 4,
		// Bottom
		0, 3, 2,
		2, 1, 0,
		// Back
		0, 1, 5,
		5, 4, 0,
		// Front
		3, 7, 6,
		6, 2, 3
];

pub const POINT_LIGHT_POSITIONS:[Vec3; 4] = [
    Vec3::new(0.7, 0.2, 2.0),
    Vec3::new(2.3, -3.3, -4.0),
    Vec3::new(-4.0, 2.0, -12.0),
    Vec3::new(0.0, 0.0, -3.0),
];

pub const SHADOW_WIDTH:i32 = 2560;
pub const SHADOW_HEIGHT:i32 = 2560;


#[rustfmt::skip]
pub const GROUND_PLANE:[f32; 36] = [
        // positions         // normals         
         100.0, 0.0,  100.0,  0.0, 1.0, 0.0,  
        -100.0, 0.0,  100.0,  0.0, 1.0, 0.0,  
        -100.0, 0.0, -100.0,  0.0, 1.0, 0.0,  
         100.0, 0.0,  100.0,  0.0, 1.0, 0.0,  
        -100.0, 0.0, -100.0,  0.0, 1.0, 0.0,  
         100.0, 0.0, -100.0,  0.0, 1.0, 0.0,  
];

pub const GRASSES:[&str;6] = [
     "resources/models/tuft/tuffft.obj",
     "resources/models/tuft/tuffft.obj",
     "resources/models/tuft/tuffft.obj",
     "resources/models/tuft/tuffft.obj",
     "resources/models/tuft/tuffft.obj",
     "resources/models/tuft/tuffft.obj",
    // "resources/models/my_obj/ground_07.obj",
    // "resources/models/my_obj/ground_06.obj",
    // "resources/models/my_obj/ground_05.obj",
    // "resources/models/my_obj/ground_04.obj",
    // "resources/models/my_obj/ground_03.obj",
    // "resources/models/my_obj/ground_02.obj",
    // "resources/models/my_obj/ground_01.obj"
];

pub const TREES:[&str; 6] = [
    "resources/models/obj/tree_small.obj",
    "resources/models/obj/tree_thin.obj",
    "resources/models/obj/tree_tall.obj",
    "resources/models/obj/tree_default.obj",
    "resources/models/obj/tree_cone.obj",
    "resources/models/obj/tree_oak_dark.obj",
];

pub const MAX_BONE_INFLUENCE: usize = 4;
pub const MAX_BONES: u32 = 200;

pub const GRAVITY: f32 = 9.81;
pub const DECREASED_GRAVITY_SCALAR: f32 = 0.5;

pub const FREEFALL_DELAY: f32 = 0.35;

pub const GROUP_TERRAIN: u32 = 0b0001;
pub const GROUP_PLAYER: u32 = 0b0010;

// Ability slot indices
pub const BASIC: u32     = 0; // LMouse
pub const DEFENSIVE: u32 = 1; // RMouse
pub const SKILL1: u32    = 2; // Q
pub const SKILL2: u32    = 3; // E
pub const EVADE: u32     = 4; // SHIFT
pub const ULTIMATE: u32  = 5; // R
