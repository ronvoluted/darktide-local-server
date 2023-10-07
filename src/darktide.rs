use sysinfo::{Pid, ProcessExt, System, SystemExt};

pub fn is_darktide_running() -> bool {
    let mut sys = System::new_all();
    sys.refresh_all();

    for (_pid, proc_) in sys.processes() {
        if proc_.name() == "Darktide.exe" {
            return true;
        }
    }

    false
}

pub fn is_process_running(pid: Pid) -> bool {
    let mut sys = System::new_all();
    sys.refresh_all();
    sys.processes().contains_key(&(pid)) // Cast to Pid type
}
