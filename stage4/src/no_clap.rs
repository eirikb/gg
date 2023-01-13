use std::env;

#[derive(Debug)]
pub struct NoClap {
    pub gg_args: Vec<String>,
    pub cmds: String,
    pub app_args: Vec<String>,
}

impl NoClap {
    pub fn new() -> Option<Self> {
        let args: Vec<String> = env::args().skip(1).collect();

        let start_at = args.iter().position(|item| !item.starts_with("-"));
        if let Some(start_at) = start_at {
            let cmds = args.get(start_at);
            if let Some(cmds) = cmds {
                let gg_args: Vec<String> = args.clone().into_iter().take(start_at).collect();
                let app_args: Vec<String> = args.clone().into_iter().skip(start_at + 1).collect();
                Some(Self { gg_args, cmds: cmds.to_string(), app_args })
            } else {
                None
            }
        } else {
            None
        }
    }
}
