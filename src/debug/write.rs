// use std::fmt::Debug;
// use std::fs::OpenOptions;
// use std::io::Write;

// pub fn write_data<T: Debug>(input: T, file_path: &str) {
//     let mut file = OpenOptions::new()
//         .create(true)
//         .append(true)
//         .open(format!("debug_out/{}", file_path).as_str())
//         .unwrap();

//     // Use pretty-print debug: "{:#?}"
//     writeln!(file, "{:#?}", input).expect("Failed to write bone debug info");
// }
