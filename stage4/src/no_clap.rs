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

#[cfg(test)]
mod test {
    // use crate::bloody_indiana_jones::download_unpack_and_all_that_stuff;

    use crate::NoClap;

    #[tokio::test]
    async fn ok() {
        // let url = String::from("https://nodejs.org/dist/v16.17.1/node-v16.17.1-linux-x64.tar.xz");
        // download_unpack_and_all_that_stuff(&url, &String::from("node")).await;

        NoClap
    }
}
