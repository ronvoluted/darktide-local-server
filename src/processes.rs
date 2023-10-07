// use sysinfo::{Pid, Process, ProcessExt, System, SystemExt};
use sysinfo::{Pid, ProcessExt, System, SystemExt};
use winapi::um::{
    handleapi::CloseHandle, processthreadsapi::OpenProcess, processthreadsapi::TerminateProcess,
    winnt::PROCESS_TERMINATE,
};

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

pub fn stop_process(pid: u32) -> bool {
    unsafe {
        let h_process = OpenProcess(PROCESS_TERMINATE, 0, pid);

        if h_process.is_null() {
            return false;
        }

        let success = TerminateProcess(h_process, 1);
        CloseHandle(h_process);

        success != 0
    }
}
