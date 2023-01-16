use std::{
    path::PathBuf,
    process::{Child, Command},
    time::Duration,
};
use fs_extra::dir::CopyOptions;

use tempdir::TempDir;

struct Setup {
    server_dir: PathBuf,
}

impl Setup {
    fn init() -> anyhow::Result<Setup> {
        let project_root = {
            let bytes = Command::new("git")
                .args(["rev-parse", "--show-toplevel"])
                .spawn()?
                .wait_with_output()?
                .stdout;

            let path = String::from_utf8(bytes)?;

            PathBuf::from(path)
        };


        let server_dir = project_root.join("test-data/server");

        println!("server directory: {server_dir:?}");

        Ok(Setup { server_dir })
    }
}

struct Server {
    #[allow(unused)]
    dir: TempDir,
    process: Child,
}

impl Drop for Server {
    fn drop(&mut self) {
        println!("killing the minecraft server...");
        let _ignored = self.process.kill();
        let _ignored = self.process.wait();
        println!("killed");
    }
}

impl Server {
    fn init() -> anyhow::Result<Server> {
        let dir = TempDir::new("mc-server")?;

        let Setup { server_dir } = Setup::init()?;

        println!("copying into {:?}..", dir.path());

        let options = CopyOptions {
            content_only: true,
            ..CopyOptions::default()
        };

        fs_extra::dir::copy(server_dir, dir.path(), &options)?;

        let java = "/Library/Java/JavaVirtualMachines/jdk1.8.0_202.jdk/Contents/Home/bin/java";

        let process = Command::new(java)
            .args(["-jar", "server.jar"])
            .current_dir(dir.path())
            .spawn()?;

        Ok(Self { dir, process })
    }
}

#[test]
fn test_run_paper() -> anyhow::Result<()> {
    let _server = Server::init()?;
    std::thread::sleep(Duration::from_secs(60));

    Ok(())
}
