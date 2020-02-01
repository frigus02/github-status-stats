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
                                {{#if grafana_org_id}}
                                    <a href=\"/_/d/builds/builds?orgId={{grafana_org_id}}\">{{full_name}}</a>
                                {{else}}
                                    {{full_name}} (waiting for first import)
                                {{/if}}
                            </li>
                        {{/each}}
                    </ul>
                    <a href=\"https://github.com/apps/status-stats\">Add repository</a>
                {{/with}}{{/if}}
                {{#if Error}}{{#with Error}}
                    <h2>Something went wrong</h2>
                    <p>{{message}}</p>
                {{/with}}{{/if}}
            </body>
            </html>
        ";
        hb.register_template_string("index.html", index_template)
            .expect("register index.html");

        hb
    };
}

#[derive(Serialize)]
pub struct RepositoryAccess {
    pub full_name: String,
    pub grafana_org_id: Option<i32>,
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
    Error {
        message: String,
    },
}

pub fn render_index(data: &IndexTemplate) -> String {
    HANDLEBARS
        .render("index.html", data)
        .unwrap_or_else(|err| err.to_string())
}
