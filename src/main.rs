use bip0039::{Count, Mnemonic};
use clap::Parser;
use comfy_table::presets::UTF8_FULL;
use comfy_table::*;
use futures::{stream, StreamExt};
use names::{Generator, Name};
use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::Rng;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::Value;
use std::collections::HashMap;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
    vec,
};
use tracing::debug;

lazy_static::lazy_static! {
    static ref CLIENT:Client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.113 Safari/537.36")
        .build()
        .unwrap();
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(short, long)]
    /// url to test on
    url: String,

    #[clap(short, long, default_value_t = 5000)]
    /// Amount of times to test
    times: usize,

    #[clap(short, long)]
    /// Weather or not this is a google form
    google: bool,

    #[clap(short, long)]
    /// Only do required parts with google forms.
    required: bool,

    #[clap(short, long, default_value_t = 8)]
    // Thread count
    parallel: usize,
}

pub enum FieldType {
    ShortText = 0,
    LongText = 1,
    MultipleChoice = 2,
    Checkbox = 3,
    Dropdown = 4,
    LinearScale = 5,
    MultiChoiceGrid = 7,
    Date = 9,
    Time = 10,
}
impl Default for FieldType {
    fn default() -> Self {
        FieldType::ShortText
    }
}
impl FieldType {
    fn new(i: i64) -> Self {
        match i {
            0 => FieldType::ShortText,
            1 => FieldType::LongText,
            2 => FieldType::MultipleChoice,
            3 => FieldType::Checkbox,
            4 => FieldType::Dropdown,
            5 => FieldType::LinearScale,
            7 => FieldType::MultiChoiceGrid,
            9 => FieldType::Date,
            10 => FieldType::Time,
            _ => FieldType::ShortText,
        }
    }
}
#[derive(Default)]
pub struct Field {
    validation: bool,
    required: bool,
    id: String,
    name: String,
    field_type: FieldType,
    choices: Vec<Choice>,
    has_choices: bool,
}

impl Field {
    pub fn new() -> Self {
        Self {
            validation: false,
            required: false,
            field_type: FieldType::LongText,
            choices: vec![],
            ..Default::default()
        }
    }

    /// Set the field's validation.
    pub fn validation(mut self, validation: bool) -> Self {
        self.validation = validation;
        self
    }
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }
    pub fn has_choices(mut self, has_choices: bool) -> Self {
        self.has_choices = has_choices;
        self
    }
    pub fn id(mut self, id: String) -> Self {
        self.id = id;
        self
    }
    pub fn name(mut self, name: String) -> Self {
        self.name = name;
        self
    }
    pub fn field_type(mut self, field_type: FieldType) -> Self {
        self.field_type = field_type;
        self
    }
    pub fn choices(mut self, choices: Vec<Choice>) -> Self {
        self.choices = choices;
        self
    }
    pub fn add_choice(mut self, choice: Choice) -> Self {
        self.choices.push(choice);
        self
    }
}

pub struct Choice {
    choice_name: String,
}

impl Choice {
    fn new(choice_name: String) -> Self {
        Self { choice_name }
    }
}

struct GoogleFormSpammer {
    form_url: String,
    required_only: bool,
    scraped_data: Vec<Field>,
}

impl GoogleFormSpammer {
    fn new(form_url: String, required_only: bool) -> Self {
        Self {
            form_url,
            required_only,
            scraped_data: Vec::new(),
        }
    }
    async fn _scrape_form(&mut self) {
        let response = CLIENT.get(&self.form_url).send().await.unwrap();

        let data = Html::parse_document(&response.text().await.unwrap())
            .select(&Selector::parse("div").unwrap())
            .filter(|x| x.value().attr("jsmodel").is_some())
            .map(|div| {
                let div_value = div.value();

                let data_params: Value = serde_json::from_str(
                    &div_value.attr("data-params").unwrap().replace("%.@.", "["),
                )
                .unwrap();

                let response_data = &data_params[0][4];
                let mut field = Field::new()
                    .field_type(FieldType::new(data_params[0][3].as_i64().unwrap()))
                    .name(data_params[0][1].as_str().unwrap_or_default().to_string())
                    .id(response_data[0][0].as_i64().unwrap().to_string())
                    .validation(response_data[0][4].as_array().unwrap().len() > 0)
                    .required(response_data[0][2].as_bool().unwrap() == true)
                    .has_choices(false);

                if response_data[0][1].as_array().unwrap().len() > 0 {
                    for raw_choices in response_data[0][1].as_array().unwrap() {
                        let choice = Choice::new(raw_choices[0].as_str().unwrap().to_owned());
                        field = field.add_choice(choice);
                    }
                    field = field.has_choices(true);
                }
                field
            })
            .collect::<Vec<Field>>();
        self.scraped_data = data;
    }
    fn generate_post_data(&self, data_length: i32) -> HashMap<String, String> {
        let mut post_data: HashMap<String, String> = HashMap::new();

        let scraped_form_data: Vec<&Field> = if self.required_only {
            self.scraped_data.iter().filter(|x| x.required).collect()
        } else {
            self.scraped_data.iter().collect()
        };

        let mut rng = thread_rng();
        let mut generator = Generator::with_naming(Name::Numbered);

        for field in scraped_form_data {
            match field.field_type {
                FieldType::Time => {
                    post_data.insert(
                        format!("entry.{}_hour", field.id),
                        format!("{}:02d", rng.gen_range(0..24)),
                    );
                    post_data.insert(
                        format!("entry.{}_minute", field.id),
                        format!("{}:02d", rng.gen_range(0..59)),
                    );
                }
                FieldType::Date => {
                    post_data.insert(
                        format!("entry.{}_year", field.id),
                        format!("{}:02d", rng.gen_range(2000..2022)),
                    );
                    post_data.insert(
                        format!("entry.{}_month", field.id),
                        format!("{}:02d", rng.gen_range(1..12)),
                    );
                    post_data.insert(
                        format!("entry.{}_day", field.id),
                        format!("{}:02d", rng.gen_range(1..29)),
                    );
                }
                _ => {
                    let name: String = if field.validation {
                        let emails = vec!["yahoo.com", "hotmail.com", "outlook.net", "gmail.com"];
                        format!(
                            "{}@{}",
                            generator.next().unwrap(),
                            emails.choose(&mut rng).unwrap()
                        )
                    } else if field.has_choices {
                        let field = field.choices.choose(&mut rng).unwrap();
                        field.choice_name.clone()
                    } else {
                        let mnemonic = Mnemonic::generate(Count::Words12);
                        mnemonic.phrase().to_owned()
                    };
                    post_data.insert(format!("entry.{}", field.id), name);
                }
            }
        }
        post_data
    }
    async fn post_data(&self) -> bool {
        let params = self.generate_post_data(50);
        let r = CLIENT
            .post(&self.form_url)
            .query(&params)
            .body(serde_json::to_string(&params).unwrap())
            .send()
            .await
            .unwrap();

        let status = r.status();
        return status.is_success();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let begin = Instant::now();
    let cli = Cli::parse();
    println!("[-] fetching {} for {} times", cli.url, cli.times);
    tracing_subscriber::fmt::init();

    let spammer = if cli.google {
        let mut spammer = GoogleFormSpammer::new(cli.url.clone(), cli.required);
        spammer._scrape_form().await;
        Some(Arc::new(spammer))
    } else {
        None
    };

    let started = Instant::now();

    let (success, failed) = (Arc::new(Mutex::new(0)), Arc::new(Mutex::new(0)));

    let urls = vec![cli.url; cli.times];
    let bodies = stream::iter(urls)
        .map(|url| {
            let (success_clone, failed_clone) = (Arc::clone(&success), Arc::clone(&failed));
            let spammer = spammer.as_ref().map(|x| Arc::clone(&x));
            async move {
                if cli.google {
                    let r = spammer.unwrap().post_data().await;
                    match r {
                        true => {
                            let mut suc = success_clone.lock().unwrap();
                            *suc += 1;
                        }
                        false => {
                            let mut fail = failed_clone.lock().unwrap();
                            *fail += 1;
                        }
                    }
                } else {
                    let r = CLIENT.get(url).send().await;
                    match r {
                        Ok(r) => {
                            let mut suc = success_clone.lock().unwrap();
                            *suc += 1;
                            debug!("req s: {}", r.status().as_u16())
                        }
                        Err(e) => {
                            let mut fail = failed_clone.lock().unwrap();
                            *fail += 1;
                            debug!("{e:?}")
                        }
                    };
                }
            }
        })
        .buffer_unordered(cli.parallel);

    bodies.for_each(|_| async {}).await;

    let (f, s) = (*failed.lock().unwrap(), *success.lock().unwrap());
    let now = Instant::now();
    let elapsed = now.duration_since(started).as_millis();
    let mut table = Table::new();

    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .add_row(vec![
            Cell::new("Time Elapsed"),
            Cell::new(format!(
                "{} total: {}",
                elapsed as f64 / 1000.0,
                now.duration_since(begin).as_millis() as f64 / 1000.0
            )),
        ])
        .add_row(vec![
            Cell::new("Speed"),
            Cell::new(format!("{} req/s", s as f64 / (elapsed as f64 / 1000.0))),
        ])
        .add_row(vec![
            Cell::new("Amount of requests"),
            Cell::new(format!(
                "total: {} succes % {}",
                s,
                if f == 0 || s == 0 { 100 } else { s / f * 100 }
            )),
        ]);
    println!("{table}");
    Ok(())
}
