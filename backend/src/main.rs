use actix::prelude::*;
use actix_files::{Files, NamedFile};
use actix_web::{get, post, web, App, HttpServer};

use std::time;
use tokio_timer;

// ---- Apis ("/api/*") ----

#[get("delayed-response/{delay}")]
fn delayed_response(
    delay: web::Path<u64>,
) -> impl Future<Item = String, Error = tokio_timer::Error> {
    tokio_timer::sleep(time::Duration::from_millis(*delay))
        .and_then(move |()| Ok(format!("Delay was set to {}ms.", delay)))
}

fn main() -> std::io::Result<()> {
    let system = System::new("odin-media-server");
    let ip = "0.0.0.0";
    let port = "20789";
    HttpServer::new(move || {
        App::new()
            .service(
                web::scope("/api/")
                    .service(delayed_response)
                    .default_service(web::route().to(web::HttpResponse::NotFound)),
            )
            .service(Files::new("/public", "./frontend/public"))
            .service(Files::new("/pkg", "./frontend/pkg"))
            .default_service(web::get().to(|| NamedFile::open("./frontend/index.html")))
    })
    .bind(format!("{}:{}", ip, port))?
    .run()?;

    println!("Done!");
    println!("Server running on: {}:{}", ip, port);
    system.run()
}
