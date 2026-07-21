use std::io;
use std::process::{Child, Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use std::thread;
use std::time::{Duration, Instant};

use crate::error::{GdxError, GdxResult};

static CANCELLED: AtomicBool = AtomicBool::new(false);
static INSTALL_HANDLER: Once = Once::new();

pub fn initialize_cancellation() {
    CANCELLED.store(false, Ordering::SeqCst);
    INSTALL_HANDLER.call_once(|| {
        let _ = ctrlc::set_handler(|| {
            CANCELLED.store(true, Ordering::SeqCst);
        });
    });
}

pub fn cancellation_requested() -> bool {
    CANCELLED.load(Ordering::SeqCst)
}

pub fn prepare_one_shot_command(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
}

pub struct ManagedChild {
    child: Child,
    #[cfg(windows)]
    job: WindowsJob,
}

impl ManagedChild {
    pub fn spawn(command: &mut Command) -> GdxResult<Self> {
        prepare_one_shot_command(command);
        let mut child = command.spawn().map_err(|err| {
            GdxError::tool("spawn_failed", format!("Cannot start process: {err}"))
        })?;
        #[cfg(windows)]
        let job = match WindowsJob::assign(&child) {
            Ok(job) => job,
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(GdxError::tool(
                    "job_object_failed",
                    format!("Cannot manage child process tree: {err}"),
                ));
            }
        };
        Ok(Self {
            child,
            #[cfg(windows)]
            job,
        })
    }

    pub fn id(&self) -> u32 {
        self.child.id()
    }

    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }

    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        self.child.wait()
    }

    pub fn terminate_tree(&mut self, grace: Duration) -> GdxResult<()> {
        self.request_graceful_exit();
        let started = Instant::now();
        while started.elapsed() < grace {
            if self.child.try_wait().ok().flatten().is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        #[cfg(windows)]
        self.job.terminate().map_err(|err| {
            GdxError::tool(
                "kill_failed",
                format!("Cannot terminate child process tree: {err}"),
            )
        })?;
        #[cfg(unix)]
        unsafe {
            libc::kill(-(self.child.id() as i32), libc::SIGKILL);
        }
        #[cfg(not(any(windows, unix)))]
        self.child.kill().map_err(|err| {
            GdxError::tool("kill_failed", format!("Cannot terminate child: {err}"))
        })?;
        let _ = self.child.wait();
        Ok(())
    }

    fn request_graceful_exit(&mut self) {
        #[cfg(windows)]
        unsafe {
            const CTRL_BREAK_EVENT: u32 = 1;
            let _ = GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, self.child.id());
        }
        #[cfg(unix)]
        unsafe {
            libc::kill(-(self.child.id() as i32), libc::SIGTERM);
        }
        #[cfg(not(any(windows, unix)))]
        {
            let _ = self.child.kill();
        }
    }
}

#[cfg(windows)]
struct WindowsJob {
    handle: isize,
}

#[cfg(windows)]
impl WindowsJob {
    fn assign(child: &Child) -> io::Result<Self> {
        use std::os::windows::io::AsRawHandle;

        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle == 0 {
                return Err(io::Error::last_os_error());
            }
            let mut info = JobObjectExtendedLimitInformation::default();
            info.basic_limit_information.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                handle,
                JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
                &info as *const _ as *const std::ffi::c_void,
                std::mem::size_of::<JobObjectExtendedLimitInformation>() as u32,
            ) == 0
            {
                CloseHandle(handle);
                return Err(io::Error::last_os_error());
            }
            if AssignProcessToJobObject(handle, child.as_raw_handle() as isize) == 0 {
                CloseHandle(handle);
                return Err(io::Error::last_os_error());
            }
            Ok(Self { handle })
        }
    }

    fn terminate(&self) -> io::Result<()> {
        unsafe {
            if TerminateJobObject(self.handle, 1) == 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }
}

#[cfg(windows)]
impl Drop for WindowsJob {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

#[cfg(windows)]
const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x0000_2000;
#[cfg(windows)]
const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS: u32 = 9;

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct JobObjectBasicLimitInformation {
    per_process_user_time_limit: i64,
    per_job_user_time_limit: i64,
    limit_flags: u32,
    minimum_working_set_size: usize,
    maximum_working_set_size: usize,
    active_process_limit: u32,
    affinity: usize,
    priority_class: u32,
    scheduling_class: u32,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct IoCounters {
    read_operation_count: u64,
    write_operation_count: u64,
    other_operation_count: u64,
    read_transfer_count: u64,
    write_transfer_count: u64,
    other_transfer_count: u64,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct JobObjectExtendedLimitInformation {
    basic_limit_information: JobObjectBasicLimitInformation,
    io_info: IoCounters,
    process_memory_limit: usize,
    job_memory_limit: usize,
    peak_process_memory_used: usize,
    peak_job_memory_used: usize,
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateJobObjectW(attributes: *const std::ffi::c_void, name: *const u16) -> isize;
    fn SetInformationJobObject(
        job: isize,
        info_class: u32,
        info: *const std::ffi::c_void,
        info_length: u32,
    ) -> i32;
    fn AssignProcessToJobObject(job: isize, process: isize) -> i32;
    fn TerminateJobObject(job: isize, exit_code: u32) -> i32;
    fn CloseHandle(handle: isize) -> i32;
    fn GenerateConsoleCtrlEvent(ctrl_event: u32, process_group_id: u32) -> i32;
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Stdio;

    #[test]
    fn job_object_terminates_spawned_grandchild() {
        let pid_path = std::env::temp_dir().join(format!(
            "gdx_grandchild_{}_{}.txt",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        let mut command = Command::new("powershell");
        command
            .args([
                "-NoProfile",
                "-Command",
                "$p = Start-Process -WindowStyle Hidden powershell -ArgumentList '-NoProfile','-Command','Start-Sleep -Seconds 30' -PassThru; Set-Content -LiteralPath $env:GDX_TEST_PID_PATH -Value $p.Id; Start-Sleep -Seconds 30",
            ])
            .env("GDX_TEST_PID_PATH", &pid_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = ManagedChild::spawn(&mut command).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !pid_path.is_file() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(25));
        }
        let grandchild_pid = fs::read_to_string(&pid_path)
            .unwrap()
            .trim()
            .parse::<u32>()
            .unwrap();

        child.terminate_tree(Duration::from_millis(250)).unwrap();

        assert!(!process_exists(grandchild_pid));
        let _ = fs::remove_file(pid_path);
    }

    fn process_exists(pid: u32) -> bool {
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ exit 1 }} else {{ exit 0 }}"
                ),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| !status.success())
            .unwrap_or(true)
    }
}
