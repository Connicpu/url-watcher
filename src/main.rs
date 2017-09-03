extern crate reqwest;

use std::io::Read;
use std::process::exit;
use std::mem::swap;
use std::thread;

fn usage(code: i32) -> ! {
    eprintln!("Usage: url-watcher [frequency (ms)] [watched url] [trigger url]");
    exit(code);
}

fn main() {
    let mut args = std::env::args();
    args.next();

    let mut first_arg = args.next().unwrap_or_else(|| usage(0));
    if first_arg == "--help" || first_arg == "-h" {
        usage(0);
    }

    let mut verbose = false;
    if first_arg == "-v" {
        verbose = true;
        first_arg = args.next().unwrap_or_else(|| usage(0));
    }

    let frequency = first_arg
        .parse()
        .map(std::time::Duration::from_millis)
        .unwrap_or_else(|_| usage(1));

    let watched_url = args.next().unwrap_or_else(|| usage(1));

    let trigger_url = args.next().unwrap_or_else(|| usage(1));

    let mut body = vec![];
    let mut prev_body = vec![];

    let mut resp = match reqwest::get(&watched_url) {
        Ok(resp) => resp,
        Err(_) => {
            eprintln!("Failed to request URL to obtain the initial body");
            exit(1);
        }
    };
    resp.read_to_end(&mut prev_body)
        .expect("Failure to read initial request body");
    drop(resp);

    let mut consecutive_failures = 0;
    let mut do_post_check = false;
    loop {
        thread::sleep(frequency);

        let mut resp = match reqwest::get(&watched_url) {
            Ok(resp) => resp,
            Err(_) => {
                consecutive_failures += 1;
                if consecutive_failures < 10 {
                    eprintln!("Failed to request watched URL, will try again");
                    continue;
                } else {
                    eprintln!("Failed to request watched URL 10 times in a row");
                    std::process::exit(1);
                }
            }
        };

        resp.read_to_end(&mut body)
            .expect("IO failure reading web request");

        let ctype = resp.headers().get::<reqwest::header::ContentType>();

        if !do_post_check && body != prev_body {
            println!("Payload bodies differ, requesting target URL");

            let client = reqwest::Client::new().unwrap();
            let mut req = client.post(&trigger_url).unwrap();
            if let Some(ctype) = ctype {
                req.header(ctype.clone());
            }

            req.body(body.clone());
            client.execute(req.build()).expect("Failed to request trigger URL");

            do_post_check = true;
        } else {
            if verbose && !do_post_check {
                println!("Bodies are the same");
            }
            do_post_check = false;
        }

        swap(&mut body, &mut prev_body);
        body.clear();
    }
}
