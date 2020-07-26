use std::path::{Path, PathBuf};
use tide::{utils::async_trait, Body, Endpoint, Request, Response, Result, Route};

struct ServeFile {
    path: PathBuf,
}

#[async_trait]
impl<State> Endpoint<State> for ServeFile
where
    State: Clone + Send + Sync + 'static,
{
    async fn call<'a>(&'a self, _req: Request<State>) -> Result {
        let res: Response = Body::from_file(&self.path).await?.into();
        Ok(res)
    }
}

pub trait RouteExt {
    fn serve_file(&mut self, path: impl AsRef<Path>);
}

impl<'a, State> RouteExt for Route<'a, State>
where
    State: Clone + Send + Sync + 'static,
{
    fn serve_file(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref().to_owned();
        self.get(ServeFile { path });
    }
}
