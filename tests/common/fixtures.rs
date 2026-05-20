use fake::faker::internet::en::SafeEmail;
use fake::faker::lorem::en::{Paragraph, Sentence};
use fake::faker::name::en::Name;
use fake::Fake;
use serde_json::{json, Value};

pub fn fake_register_payload() -> Value {
    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    json!({
        "name": name,
        "email": email,
        "password": "qwerty123",
    })
}

pub fn fake_post_payload() -> Value {
    let title: String = Sentence(3..6).fake();
    let body: String = Paragraph(2..4).fake();
    let status = if rand::random::<bool>() {
        "draft"
    } else {
        "published"
    };
    json!({
        "title": title.trim_end_matches('.'),
        "content": body,
        "status": status,
    })
}

pub fn fake_comment_payload() -> Value {
    let body: String = Sentence(5..12).fake();
    json!({
        "comment": body,
    })
}
