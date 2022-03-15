# Google Form Tester

Test your form by creating mocking data

## 💳 Credits

This project is a rust rewrite of [GoogleFormSpammer](https://github.com/IlluminatiFish/GoogleFormSpammer) by [IlluminatiFish](https://github.com/IlluminatiFish).

## Features

- Create mocking data for your google forms
- Test your api's with 1k r/s

## 💥 Usage

```sh
google_form_tester 0.1.0

USAGE:
    google_form_tester [OPTIONS] --url <URL>

OPTIONS:
    -g, --google                 Weather or not this is a google form
    -h, --help                   Print help information
    -p, --parallel <PARALLEL>    Thread/Parallel count 50 recommended for fastest speeds [default:
                                 8]
    -r, --required               Only do required parts with google forms
    -t, --times <TIMES>          Amount of times to test [default: 5000]
    -u, --url <URL>              url to test on
    -V, --version                Print version information
```

## Disclaimer

I am not liable for any malicious activity when people use this program
