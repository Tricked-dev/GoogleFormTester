use bip0039::{Count, Mnemonic};
use names::{Generator, Name};
use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::Rng;
use scraper::{Html, Selector};
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

use crate::client::CLIENT;
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

pub struct GoogleFormSpammer {
    form_url: String,
    required_only: bool,
    scraped_data: Vec<Field>,
}

impl GoogleFormSpammer {
    pub fn new(form_url: String, required_only: bool) -> Self {
        Self {
            form_url,
            required_only,
            scraped_data: Vec::new(),
        }
    }
    pub async fn _scrape_form(&mut self) {
        let mut response = CLIENT.get(&self.form_url).send().await.unwrap();

        let data = Html::parse_document(&response.body_string().await.unwrap())
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
                    .validation(!response_data[0][4].as_array().unwrap().is_empty())
                    .required(response_data[0][2].as_bool().unwrap())
                    .has_choices(false);

                if !response_data[0][1].as_array().unwrap().is_empty() {
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
    fn generate_post_data(&self, _data_length: i32) -> HashMap<String, String> {
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
    pub async fn post_data(&self) -> bool {
        let params = self.generate_post_data(50);
        let r = CLIENT
            .post(&self.form_url)
            .header("content-length", "0")
            .query(&params)
            .unwrap()
            .send()
            .await;
        match r {
            Ok(r) => {
                let status = r.status();
                dbg!(status);
                debug!("req s: {:?}", status);
                status.is_success()
            }
            Err(_) => false,
        }
    }
}
