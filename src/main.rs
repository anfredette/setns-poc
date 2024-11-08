use clap::{Args, Parser, Subcommand};
use core::str;
use nix::sched::{setns, CloneFlags};
use std::fs;
use std::time::Duration;
use std::{fs::File, process};
use std::{thread, time};
// use tokio;

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Mount(MountArgs),
    Net(NetArgs),
}

#[derive(Debug, Args)]
struct MountArgs {
    /// Required: The PID of the target process.
    #[clap(short, long)]
    pid: u32,

    /// Optional: If true, start some threads before calling setns to make the
    /// calling process multi-threaded. This is to test if the setns call works
    /// with multi-threaded processes.
    /// [default: false]
    #[clap(short, long)]
    multi_thread: bool,
}

#[derive(Debug, Args)]
struct NetArgs {
    /// The name of the target network namespace.
    /// Only one of --name or --pid can be provided.
    #[clap(short = 'n', long)]
    name: Option<String>,

    /// The PID of a process using the target network namespace.
    /// Only one of --name or --pid can be provided.
    #[clap(short = 'p', long)]
    pid: Option<u32>,

    /// Optional: If true, start some threads before calling setns to make the
    /// calling process multi-threaded. This is to test if the setns call works
    /// with multi-threaded processes.
    /// [default: false]
    #[clap(short, long)]
    multi_thread: bool,
}

fn main() {
    let main_pid = process::id();
    println!("The PID in main is: {}", main_pid);

    let cli = Cli::parse();
    match cli.command {
        Commands::Mount(args) => {
            if args.multi_thread {
                start_threads();
            }
            mnt_ns_test(args.pid);
        }
        Commands::Net(args) => {
            if args.name.is_none() && args.pid.is_none() {
                println!("Error: Either --netns-name or --netns-pid must be provided.");
                return;
            }
            if args.multi_thread {
                start_threads();
            }
            if let Some(pid) = args.pid {
                let path = format!("/proc/{pid}/ns/net");
                netns_test(path);
            } else if let Some(name) = &args.name {
                let path = format!("/var/run/netns/{name}");
                netns_test(path);
            }
        }
    }

    //print_message("main() waiting...".to_string())
}

fn mnt_ns_test(target_pid: u32) {
    let setns_test_pid = process::id();
    println!("The setns_test PID is: {}", setns_test_pid);
    println!("The target PID is: {}", target_pid);

    let bpfd_mnt_file_path = format!("/proc/{setns_test_pid}/ns/mnt");
    let target_mnt_file_path = format!("/proc/{target_pid}/ns/mnt");

    println!(
        "bpfd_mnt_file_path: {:?}, target_mnt_file_path: {:?}",
        bpfd_mnt_file_path, target_mnt_file_path
    );

    let bpfd_mnt_file = match File::open(bpfd_mnt_file_path) {
        Ok(file) => file,
        Err(e) => panic!("error opening bpfd file: {}", e),
    };

    let target_mnt_file = match File::open(target_mnt_file_path) {
        Ok(file) => file,
        Err(e) => panic!("error opening target file: {}", e),
    };

    ls_dir();

    let setns_result = setns(target_mnt_file, CloneFlags::CLONE_NEWNS);
    println!("target setns: {:?}", setns_result);

    ls_dir();

    let setns_result = setns(bpfd_mnt_file, CloneFlags::CLONE_NEWNS);
    println!("bpfd setns: {:?}", setns_result);

    ls_dir();
}

fn netns_test(target_netns_path: String) {
    let netns_test_pid = process::id();
    let current_netns_path = format!("/proc/{netns_test_pid}/ns/net");

    println!(
        "current_netns_path: {:?}, target_netns_path: {:?}",
        current_netns_path, target_netns_path
    );

    let current_netns_file = match File::open(current_netns_path) {
        Ok(file) => file,
        Err(e) => panic!("error opening current_netns_file: {}", e),
    };

    let target_netns_file = match File::open(target_netns_path) {
        Ok(file) => file,
        Err(e) => panic!("error opening target_netns_file: {}", e),
    };

    println!("\nInfo from base netns:\n");
    net_cmd();

    println!("\nSwitch to target netns:\n");

    let setns_result = setns(target_netns_file, CloneFlags::CLONE_NEWNET);
    println!("target setns: {:?}", setns_result);

    println!("\nInfo from target netns:\n");
    net_cmd();

    println!("\nSwitch back to base netns:\n");

    let setns_result = setns(current_netns_file, CloneFlags::CLONE_NEWNET);
    println!("bpfd setns: {:?}", setns_result);

    println!("\nInfo from base netns:\n");
    net_cmd();
}

fn start_threads() {
    thread::spawn(|| {
        print_message("thread::spawn message 1".to_string());
    });

    thread::spawn(|| {
        print_message("thread::spawn message 2".to_string());
    });

    // Wait a bit to make sure the threads are running.
    thread::sleep(time::Duration::from_secs(2));
}

fn print_message(message: String) {
    let bpfd_pid = process::id();
    println!("The print_message PID is: {}", bpfd_pid);

    for i in 1..21 {
        println!("{} #{}", message, i);
        thread::sleep(Duration::from_secs(1));
    }
}

fn ls_dir() {
    let paths = fs::read_dir("/").unwrap();

    for path in paths {
        println!("Name: {}", path.unwrap().path().display())
    }
}

fn net_cmd() {
    let output = process::Command::new("ip")
        .arg("-o")
        .arg("link")
        .output()
        .expect("Failed to execute command");

    let output_str = str::from_utf8(&output.stdout).expect("Invalid UTF-8 sequence");

    for line in output_str.lines() {
        if let Some((iface, status)) = parse_interface_line(line) {
            println!("{} {}", iface, status);
        }
    }
}

fn parse_interface_line(line: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 3 {
        let iface = parts[1].trim_end_matches(':');
        let status = if parts.contains(&"UP") { "UP" } else { "DOWN" };
        return Some((iface, status));
    }
    None
}
