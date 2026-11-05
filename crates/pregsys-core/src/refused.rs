//! An avatar VRChat refused to change to. It keeps the old avatar and logs
//! "inaccessible avatar", but its OSCQuery goes on reporting the refused ID, so
//! that ID can't be believed until VRChat has really moved on.

#[derive(Debug, Default)]
pub struct Refused {
    id: Option<String>,
}

impl Refused {
    /// VRChat's log says it refused this avatar.
    pub fn set(&mut self, id: &str) {
        self.id = Some(id.to_string());
    }

    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// VRChat reports this avatar as worn: is that true? Any other avatar means
    /// it has moved on.
    pub fn believe(&mut self, id: &str) -> bool {
        if self.id.as_deref() == Some(id) {
            return false;
        }
        self.id = None;
        true
    }

    /// The log says you're switching to this avatar name. If that's the refused
    /// avatar (you can wear it now), or its name isn't known, stop doubting it.
    pub fn on_switch(&mut self, avatar_name: &str, refused_name: Option<&str>) {
        if refused_name.is_none_or(|n| n == avatar_name) {
            self.id = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_refused_believes_everything() {
        assert!(Refused::default().believe("a"));
    }

    #[test]
    fn refused_is_doubted_until_another_avatar_is_reported() {
        let mut r = Refused::default();
        r.set("bad");
        assert!(!r.believe("bad"));
        assert!(!r.believe("bad"), "OSCQuery keeps saying it");
        assert!(r.believe("good"));
        assert_eq!(r.id(), None);
        assert!(r.believe("bad"), "moved on: it may be real now");
    }

    #[test]
    fn switching_to_another_name_keeps_the_doubt() {
        let mut r = Refused::default();
        r.set("bad");
        // OSCQuery still says "bad" while the other avatar loads.
        r.on_switch("Other", Some("Bad Avatar"));
        assert!(!r.believe("bad"));
    }

    #[test]
    fn switching_to_the_refused_avatar_ends_the_doubt() {
        let mut r = Refused::default();
        r.set("bad");
        r.on_switch("Bad Avatar", Some("Bad Avatar"));
        assert!(r.believe("bad"));

        r.set("bad");
        r.on_switch("Whatever", None);
        assert!(r.believe("bad"), "name unknown: can't tell, so trust VRChat");
    }
}
