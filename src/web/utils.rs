use maud::{html, Markup, DOCTYPE};
use surf::http::mime;
use surf::StatusCode;
use tide::Response;

/// Wraps the body into a complete HTML document.
pub fn respond_with_body(body: Markup) -> tide::Result {
    let markup: Markup = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                title { "Blitz Dashboard" }
                meta name="viewport" content="width=device-width, initial-scale=1";
                meta charset="UTF-8";
                link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bulma@0.9.2/css/bulma.min.css" crossorigin="anonymous" referrerpolicy="no-referrer";
                link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/5.15.3/css/all.min.css" integrity="sha512-iBBXm8fW90+nuLcSKlbmrPcLa0OT92xO1BIsZ+ywDWZCvqsWgccV3gFoRBv0z+8dLJgyAHIhR35VZc2oM/gI1w==" crossorigin="anonymous" referrerpolicy="no-referrer";
            }
            body {
                (body)
            }
        }
    };
    Ok(Response::builder(StatusCode::Ok)
        .body(markup.into_string())
        .content_type(mime::HTML)
        .build())
}
