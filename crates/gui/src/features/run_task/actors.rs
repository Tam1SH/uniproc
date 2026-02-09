use crate::core::actor::traits::{Context, Handler, Message};
use crate::{messages, AppWindow, RunTaskDialog};
use i_slint_backend_winit::WinitWindowAccessor;
use slint::{ComponentHandle, SharedString};

messages! {
    Show,
    Hide,
    Drag,
    Execute { env_id: SharedString, command: SharedString }
}

pub struct RunTaskActor {
    pub window: RunTaskDialog,
}

impl Handler<Drag, AppWindow> for RunTaskActor {
    fn handle(&mut self, _msg: Drag, ctx: &Context<Self, AppWindow>) {
        self.window.window().with_winit_window(|w| {
            let _ = w.drag_window();
        });
    }
}

impl Handler<Show, AppWindow> for RunTaskActor {
    fn handle(&mut self, _msg: Show, _ctx: &Context<Self, AppWindow>) {
        // let envs = vec![
        //     RunEnv {
        //         name: "Command Prompt".into(),
        //         id: "cmd".into(),
        //         icon: slint::Image::default(),
        //     },
        //     RunEnv {
        //         name: "PowerShell".into(),
        //         id: "pwsh".into(),
        //         icon: slint::Image::default(),
        //     },
        // ];
        // self.window.set_envs(ModelRc::new(VecModel::from(envs)));
        self.window.show().unwrap();
    }
}

impl Handler<Hide, AppWindow> for RunTaskActor {
    fn handle(&mut self, _msg: Hide, _ctx: &Context<Self, AppWindow>) {
        let _ = self.window.hide();
    }
}

impl Handler<Execute, AppWindow> for RunTaskActor {
    fn handle(&mut self, msg: Execute, ctx: &Context<Self, AppWindow>) {
        let cmd_str = msg.command.to_string();
        // let env_id = msg.env_id.as_str();
        //

        // std::thread::spawn(move || {
        //     let _ = match env_id {
        //         "pwsh" => Command::new("powershell")
        //             .arg("-Command")
        //             .arg(&cmd_str)
        //             .spawn(),
        //         "cmd" => Command::new("cmd").arg("/c").arg(&cmd_str).spawn(),
        //         _ if env_id.starts_with("wsl:") => {
        //             let distro = env_id.trim_start_matches("wsl:");
        //             Command::new("wsl")
        //                 .arg("-d")
        //                 .arg(distro)
        //                 .arg("-e")
        //                 .arg("sh")
        //                 .arg("-c")
        //                 .arg(&cmd_str)
        //                 .spawn()
        //         }
        //         _ => Command::new(&cmd_str).spawn(),
        //     };
        // });

        ctx.addr().send(Hide);
    }
}
