#[macro_export]
#[cfg(feature = "gl_debug")]
macro_rules! gl_call {
    ($e:expr) => {{
        while gl::GetError() != gl::NO_ERROR {} // Clear all existing OpenGL errors
        let result = $e; // Execute the OpenGL call
        $crate::macros::gl_log_call(stringify!($e), file!(), line!()); // Log any errors
        result
    }};
}

#[cfg(feature = "gl_debug")]
pub fn gl_log_call(function: &str, file: &str, line: u32) -> bool {
    unsafe {

        let mut had_error = false;
        loop {
            let err = gl::GetError();
            if err == gl::NO_ERROR {
                break; // Exit if no more errors
            }
            had_error = true;
            eprintln!(
                "[OpenGL Error] ({}) in function `{}`, at {}:{}",
                err, function, file, line
            );
        }
        had_error
    }
}

#[allow(dead_code)]
pub static mut DRAW_CALLS: usize = 0;

#[macro_export]
#[cfg(not(feature = "gl_debug"))]
macro_rules! gl_call {
    ($e:expr) => {{
        let result = $e;
        #[cfg(feature = "track_draw_calls")]
        unsafe {
            use gl::types::*;
            match stringify!($e) {
                // crude match; just count known draw calls
                s if s.contains("DrawElements") || s.contains("DrawArrays") => {
                    $crate::macros::incr_draw_call();
                }
                _ => {}
            }
        }
        result
    }};
}

#[allow(dead_code)]
pub fn incr_draw_call() {
    unsafe { DRAW_CALLS += 1; }
}
