//! Native probe modes executed through the real `opi-sandbox run` path.

use std::env;
use std::fs;
use std::net::TcpListener;
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

pub(crate) const TEST_NAME: &str = "policy_probe::native_policy_probe";

#[test]
#[ignore = "invoked as a native target by the platform policy tests"]
fn native_policy_probe() {
    match env::var("OPI_POLICY_PROBE_MODE")
        .expect("probe mode must be inherited")
        .as_str()
    {
        "inet-bind" => probe_inet_bind(),
        "unix-socket" => probe_unix_socket(),
        "environment" => probe_environment(),
        "descriptors-present" => probe_descriptors(false),
        "descriptors-filtered" => probe_descriptors(true),
        #[cfg(target_os = "linux")]
        "seccomp-status" => probe_seccomp_status(),
        #[cfg(target_os = "linux")]
        "syscall" => probe_syscall(),
        #[cfg(target_os = "linux")]
        "syscall-observe" => probe_syscall_observe(),
        mode => panic!("unknown native policy probe mode: {mode}"),
    }
}

fn probe_inet_bind() {
    match TcpListener::bind(("127.0.0.1", 0)) {
        Ok(listener) => {
            let address = listener.local_addr().expect("bound address");
            println!("INET_BIND_OK:{address}");
        }
        Err(error) => {
            eprintln!("INET_BIND_DENIED:{error}");
            std::process::exit(77);
        }
    }
}

#[cfg(unix)]
fn probe_unix_socket() {
    match UnixStream::pair() {
        Ok((_left, _right)) => println!("AF_UNIX_OK"),
        Err(error) => {
            eprintln!("AF_UNIX_FAILED:{error}");
            std::process::exit(77);
        }
    }
}

#[cfg(not(unix))]
fn probe_unix_socket() {
    panic!("Unix socket probe requires a Unix target");
}

fn probe_environment() {
    assert_eq!(
        env::var("OPI_POLICY_INHERITED").as_deref(),
        Ok("inherited-exactly")
    );
    let aliases = ["TMPDIR", "TMP", "TEMP"].map(|name| {
        env::var_os(name)
            .map(PathBuf::from)
            .unwrap_or_else(|| panic!("{name} must be set"))
    });
    assert_eq!(aliases[0], aliases[1], "TMPDIR and TMP must agree");
    assert_eq!(aliases[0], aliases[2], "TMPDIR and TEMP must agree");
    assert!(aliases[0].is_dir(), "private temp root must exist");
    let inherited_temp =
        PathBuf::from(env::var_os("OPI_POLICY_REQUEST_TEMP").expect("inherited temp alias value"));
    assert_ne!(
        aliases[0], inherited_temp,
        "runner must replace inherited temp aliases"
    );
    for (index, directory) in aliases.iter().enumerate() {
        let marker = directory.join(format!("environment-alias-{index}"));
        fs::write(&marker, b"ok").expect("private temp alias must be writable");
        assert_eq!(fs::read(&marker).expect("read alias marker"), b"ok");
    }
    println!("ENVIRONMENT_ALIASES_OK");
}

fn probe_descriptors(filtered: bool) {
    let file_fd = inherited_i32("OPI_POLICY_FILE_FD");
    let inet_fd = inherited_i32("OPI_POLICY_INET_FD");
    let unix_fd = inherited_i32("OPI_POLICY_UNIX_FD");
    let expected_file = env::var_os("OPI_POLICY_FILE_LINK").expect("file link");
    let expected_inet = env::var_os("OPI_POLICY_INET_LINK").expect("INET link");
    let expected_unix = env::var_os("OPI_POLICY_UNIX_LINK").expect("Unix link");

    let actual_file = descriptor_link(file_fd);
    let actual_inet = descriptor_link(inet_fd);
    let actual_unix = descriptor_link(unix_fd);
    if filtered {
        assert_ne!(
            actual_file.as_deref(),
            Some(expected_file.as_os_str()),
            "exact inherited file descriptor must be removed"
        );
        assert_ne!(
            actual_inet.as_deref(),
            Some(expected_inet.as_os_str()),
            "exact inherited INET descriptor must be removed"
        );
        assert_eq!(
            actual_unix.as_deref(),
            Some(expected_unix.as_os_str()),
            "inherited AF_UNIX descriptor must be preserved"
        );
        println!("EXACT_DESCRIPTORS_FILTERED");
    } else {
        assert_eq!(actual_file.as_deref(), Some(expected_file.as_os_str()));
        assert_eq!(actual_inet.as_deref(), Some(expected_inet.as_os_str()));
        assert_eq!(actual_unix.as_deref(), Some(expected_unix.as_os_str()));
        println!("EXACT_DESCRIPTORS_PRESENT");
    }
}

fn inherited_i32(name: &str) -> i32 {
    env::var(name)
        .unwrap_or_else(|_| panic!("{name} must be inherited"))
        .parse()
        .unwrap_or_else(|_| panic!("{name} must be an fd"))
}

fn descriptor_link(fd: i32) -> Option<std::ffi::OsString> {
    fs::read_link(format!("/proc/self/fd/{fd}"))
        .ok()
        .map(PathBuf::into_os_string)
}

#[cfg(target_os = "linux")]
fn probe_seccomp_status() {
    let status = fs::read_to_string("/proc/self/status").expect("read /proc/self/status");
    let mode = proc_status_u32(&status, "Seccomp");
    let filters = proc_status_u32(&status, "Seccomp_filters");
    println!("SECCOMP_STATUS:{mode}:{filters}");
}

#[cfg(target_os = "linux")]
fn proc_status_u32(status: &str, field: &str) -> u32 {
    status
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name == field {
                value.trim().parse().ok()
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("/proc/self/status has no numeric {field} field"))
}

#[cfg(target_os = "linux")]
fn probe_syscall() {
    let name = env::var("OPI_POLICY_SYSCALL").expect("syscall name");
    let (result, errno) = observe_syscall(&name);
    if result == -1 && errno == libc::EPERM {
        eprintln!("SYSCALL_DENIED:{name}:{}", libc::EPERM);
        std::process::exit(77);
    }
    println!("SYSCALL_OK:{name}:{result}:{errno}");
}

#[cfg(target_os = "linux")]
fn probe_syscall_observe() {
    let name = env::var("OPI_POLICY_SYSCALL").expect("syscall name");
    let (result, errno) = observe_syscall(&name);
    println!("SYSCALL_OBSERVED:{name}:{result}:{errno}");
}

#[cfg(target_os = "linux")]
fn observe_syscall(name: &str) -> (libc::c_long, i32) {
    let arguments = if name == "unshare_zero" {
        [0; 6]
    } else {
        [usize::MAX, 0, 0, 0, 0, 0]
    };
    let result = raw_syscall(syscall_number(name), arguments);
    let errno = if result == -1 {
        std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or_default()
    } else {
        // A successful descriptor-returning call must not leak from the probe.
        close_fd(result as i32);
        0
    };
    (result, errno)
}

#[cfg(target_os = "linux")]
fn syscall_number(name: &str) -> libc::c_long {
    match name {
        "open_by_handle_at" => libc::SYS_open_by_handle_at,
        "bpf" => libc::SYS_bpf,
        "perf_event_open" => libc::SYS_perf_event_open,
        "ptrace" => libc::SYS_ptrace,
        "kexec_load" => libc::SYS_kexec_load,
        "kexec_file_load" => libc::SYS_kexec_file_load,
        "reboot" => libc::SYS_reboot,
        "init_module" => libc::SYS_init_module,
        "finit_module" => libc::SYS_finit_module,
        "delete_module" => libc::SYS_delete_module,
        "swapon" => libc::SYS_swapon,
        "swapoff" => libc::SYS_swapoff,
        "acct" => libc::SYS_acct,
        "settimeofday" => libc::SYS_settimeofday,
        #[cfg(target_arch = "x86_64")]
        "iopl" => libc::SYS_iopl,
        #[cfg(target_arch = "x86_64")]
        "ioperm" => libc::SYS_ioperm,
        "io_uring_setup" => libc::SYS_io_uring_setup,
        "unshare_zero" => libc::SYS_unshare,
        unknown => panic!("unknown syscall probe: {unknown}"),
    }
}

#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
fn raw_syscall(number: libc::c_long, arguments: [usize; 6]) -> libc::c_long {
    // SAFETY: this is a disposable, restricted test child. Privileged probes
    // use deliberately invalid pointer/identifier arguments; the unshare
    // control uses zero flags and therefore requests no namespace changes.
    unsafe {
        libc::syscall(
            number,
            arguments[0],
            arguments[1],
            arguments[2],
            arguments[3],
            arguments[4],
            arguments[5],
        )
    }
}

#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
fn close_fd(fd: i32) {
    // SAFETY: `fd` is a nonnegative descriptor returned by the raw syscall.
    let _ = unsafe { libc::close(fd) };
}
