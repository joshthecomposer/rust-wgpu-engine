// use glam::Vec4;

// pub fn hex_to_vec4(input: &str) -> Vec4 {
//     let sanitized = if input.starts_with('#') {
//         &input[1..]
//     } else {
//         input
//     };

//     let split: Vec<String> = sanitized
//         .chars()
//         .collect::<Vec<char>>()
//         .chunks(2)
//         .map(|c| c.iter().collect::<String>())
//         .collect();

//     let mut result = Vec::new();

//     for v in split.iter() {
//         result.push(u8::from_str_radix(v, 16).unwrap() as f32)
//     }

//     Vec4::new(
//         *result.get(0).unwrap() / 255.0,
//         *result.get(1).unwrap() / 255.0,
//         *result.get(2).unwrap() / 255.0,
//         1.0,
//     )
// }
