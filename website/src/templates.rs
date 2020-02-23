use handlebars::Handlebars;
use serde::Serialize;

lazy_static! {
    static ref HANDLEBARS: Handlebars<'static> = {
        let mut hb = Handlebars::new();

        let index_template = "
            <!DOCTYPE html>
            <html>
            <head>
                <title>GitHub Status Stats</title>
            </head>
            <body>
                <h1>GitHub Status Stats</h1>
                {{#if Anonymous}}{{#with Anonymous}}
                    <div><a href={{login_url}}>Login</a></div>
                {{/with}}{{/if}}
                {{#if LoggedIn}}{{#with LoggedIn}}
                    <h2>Hello {{user}}!</h2>
                    <ul>
                        {{#each repositories}}
                            <li>
                                <a href=\"/d/{{name}}\">{{name}}</a>
                            </li>
                        {{/each}}
                    </ul>
                    <a href=\"https://github.com/apps/status-stats\">Add repository</a>
                {{/with}}{{/if}}
            </body>
            </html>
        ";
        hb.register_template_string("index.html", index_template)
            .expect("register index.html");

        let dashboard_template = "
            <!DOCTYPE html>
            <html>
            <head>
                <title>{{name}} - GitHub Status Stats</title>
            </head>
            <body>
                <h1>GitHub Status Stats</h1>
                {{#if data.Data}}{{#with data.Data}}
                    <h2>{{name}}</h2>
                    <p>{{user}}</p>
                    <p>{{repo_id}}</p>
                    <p>TODO: Dashboard...</p>
                {{/with}}{{/if}}
                {{#if data.Error}}{{#with data.Error}}
                    <h2>Something went wrong</h2>
                    <p>{{message}}</p>
                {{/with}}{{/if}}
            </body>
            </html>
        ";
        hb.register_template_string("dashboard.html", dashboard_template)
            .expect("register dashboard.html");

        hb
    };
}

#[derive(Serialize)]
pub struct RepositoryAccess {
    pub name: String,
}

#[derive(Serialize)]
pub enum IndexTemplate {
    Anonymous {
        login_url: String,
    },
    LoggedIn {
        user: String,
        repositories: Vec<RepositoryAccess>,
    },
}

pub fn render_index(data: &IndexTemplate) -> String {
    HANDLEBARS
        .render("index.html", data)
        .unwrap_or_else(|err| err.to_string())
}

#[derive(Serialize)]
pub enum DashboardData {
    Data { user: String, repo_id: i32 },
    Error { message: String },
}

#[derive(Serialize)]
pub struct DashboardTemplate {
    pub name: String,
    pub data: DashboardData,
}

pub fn render_dashboard(data: &DashboardTemplate) -> String {
    HANDLEBARS
        .render("dashboard.html", data)
        .unwrap_or_else(|err| err.to_string())
}
