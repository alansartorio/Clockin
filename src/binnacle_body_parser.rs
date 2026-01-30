use thiserror::Error;

pub struct OwnedBody {
    pub sub_project: Option<String>,
    pub subject: String,
}

impl<'a> Body<'a> {
    pub fn to_owned(&self) -> OwnedBody {
        OwnedBody {
            sub_project: self.sub_project.map(|s| s.to_owned()),
            subject: self.subject.to_owned(),
        }
    }
}

pub struct Body<'a> {
    pub sub_project: Option<&'a str>,
    pub subject: &'a str,
}

#[derive(Error, Debug)]
pub enum ParseError {}

pub struct SessionWithBody<Session> {
    pub session: Session,
    pub body: OwnedBody,
}

pub fn parse(body_str: &str) -> Result<Body<'_>, ParseError> {
    match body_str.find(": ") {
        Some(colon_idx) => Ok(Body {
            sub_project: Some(&body_str[..colon_idx]),
            subject: &body_str[colon_idx + 2..],
        }),
        None => Ok(Body {
            sub_project: None,
            subject: body_str,
        }),
    }
}
