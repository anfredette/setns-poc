use nix::sched::{setns, CloneFlags};
use std::fs;
use std::{fs::File, process};

fn main() {
    println!("Hello, world!");

    let bpfd_pid = process::id();
    let target_pid = 3141692;

    let bpfd_mnt_file_path = format!("/proc/{bpfd_pid}/ns/mnt");
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

fn ls_dir() {
    let paths = fs::read_dir("/").unwrap();

    for path in paths {
        println!("Name: {}", path.unwrap().path().display())
    }
}
