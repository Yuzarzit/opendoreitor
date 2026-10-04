//! Modo eco: baja la prioridad del proceso de la app.
//!
//! Nota: en Windows la prioridad se aplica a este proceso y a los hijos que
//! cree DESPUES; los procesos de WebView2 ya lanzados mantienen la suya.

#[cfg(target_os = "windows")]
pub fn apply(eco: bool) -> Result<(), String> {
    use std::ffi::c_void;
    type Handle = *mut c_void;

    const IDLE_PRIORITY_CLASS: u32 = 0x0000_0040;
    const NORMAL_PRIORITY_CLASS: u32 = 0x0000_0020;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> Handle;
        fn SetPriorityClass(h: Handle, class: u32) -> i32;
    }

    // SAFETY: GetCurrentProcess devuelve un pseudo-handle siempre valido.
    let ok = unsafe {
        let class = if eco { IDLE_PRIORITY_CLASS } else { NORMAL_PRIORITY_CLASS };
        SetPriorityClass(GetCurrentProcess(), class)
    };
    if ok == 0 {
        return Err("SetPriorityClass fallo".into());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn apply(eco: bool) -> Result<(), String> {
    extern "C" {
        fn setpriority(which: i32, who: u32, prio: i32) -> i32;
    }
    const PRIO_PROCESS: i32 = 0;

    // En Linux la prioridad es POR HILO: hay que recorrer todos los del proceso.
    let nice = if eco { 10 } else { 0 };
    let mut failed = false;
    let tasks = std::fs::read_dir("/proc/self/task").map_err(|e| e.to_string())?;
    for t in tasks.flatten() {
        if let Some(tid) = t.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) {
            // SAFETY: llamada FFI simple con argumentos validos.
            if unsafe { setpriority(PRIO_PROCESS, tid, nice) } != 0 {
                failed = true;
            }
        }
    }
    if failed && !eco {
        // Un usuario sin privilegios no puede volver a bajar el valor "nice".
        return Err("Linux no permite recuperar la prioridad normal sin privilegios; reinicia la app".into());
    }
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn apply(_eco: bool) -> Result<(), String> {
    Ok(())
}
