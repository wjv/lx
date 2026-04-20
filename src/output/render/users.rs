use nu_ansi_term::Style;
use uzers::Users;

use crate::fs::fields as f;
use crate::output::cell::TextCell;

impl f::User {
    /// Render the owner as their name, falling back to the numeric
    /// UID if the name can't be resolved.  Used for the `--user` column.
    pub fn render<C: Colours, U: Users>(self, colours: &C, users: &U) -> TextCell {
        let user_name = match users.get_user_by_uid(self.0) {
            None => self.0.to_string(),
            Some(user) => user.name().to_string_lossy().into(),
        };

        TextCell::paint(self.style(colours, users), user_name)
    }

    /// Render the owner as the raw numeric UID.  Used for the
    /// `--uid` column.  Uses the dedicated `uid_*` style slots so
    /// themes can distinguish it visually from the name column while
    /// still cascading from the `user_*` slots when unset.
    pub fn render_uid<C: Colours, U: Users>(self, colours: &C, users: &U) -> TextCell {
        TextCell::paint(self.uid_style(colours, users), self.0.to_string())
    }

    fn style<C: Colours, U: Users>(self, colours: &C, users: &U) -> Style {
        if users.get_current_uid() == self.0 {
            colours.you()
        } else {
            colours.someone_else()
        }
    }

    fn uid_style<C: Colours, U: Users>(self, colours: &C, users: &U) -> Style {
        if users.get_current_uid() == self.0 {
            colours.uid_you()
        } else {
            colours.uid_someone_else()
        }
    }
}

pub trait Colours {
    fn you(&self) -> Style;
    fn someone_else(&self) -> Style;

    /// Style for the numeric UID column, for the current user.
    /// Implementations should cascade from `you()` when no explicit
    /// `uid_you` slot is set.
    fn uid_you(&self) -> Style;

    /// Style for the numeric UID column, for a different user.
    /// Cascades from `someone_else()` when unset.
    fn uid_someone_else(&self) -> Style;
}

#[cfg(test)]
#[allow(unused_results)]
pub mod test {
    use super::Colours;
    use crate::fs::fields as f;
    use crate::output::cell::TextCell;

    use nu_ansi_term::Color::*;
    use nu_ansi_term::Style;
    use uzers::User;
    use uzers::mock::MockUsers;

    /// Cascading test colours: the `uid_*` methods return slightly
    /// different styles than the `user_*` methods, so the tests can
    /// verify that `render_uid` picks up the UID slots.  Real cascade
    /// (falling back to user/group when unset) is covered by the
    /// theme-level tests.
    struct TestColours;

    impl Colours for TestColours {
        fn you(&self) -> Style {
            Red.bold()
        }
        fn someone_else(&self) -> Style {
            Blue.underline()
        }
        fn uid_you(&self) -> Style {
            Red.normal()
        }
        fn uid_someone_else(&self) -> Style {
            Blue.normal()
        }
    }

    #[test]
    fn named() {
        let mut users = MockUsers::with_current_uid(1000);
        users.add_user(User::new(1000, "enoch", 100));

        let user = f::User(1000);
        let expected = TextCell::paint_str(Red.bold(), "enoch");
        assert_eq!(expected, user.render(&TestColours, &users));

        // render_uid uses the UID-specific style slot.
        let expected = TextCell::paint_str(Red.normal(), "1000");
        assert_eq!(expected, user.render_uid(&TestColours, &users));
    }

    #[test]
    fn unnamed_falls_back_to_uid() {
        let users = MockUsers::with_current_uid(1000);

        let user = f::User(1000);
        // Name column falls back to numeric UID but keeps the name slot's
        // style (Red.bold()).
        let expected_name = TextCell::paint_str(Red.bold(), "1000");
        assert_eq!(expected_name, user.render(&TestColours, &users));
        // UID column always uses the UID slot's style (Red.normal()).
        let expected_uid = TextCell::paint_str(Red.normal(), "1000");
        assert_eq!(expected_uid, user.render_uid(&TestColours, &users));
    }

    #[test]
    fn different_named() {
        let mut users = MockUsers::with_current_uid(0);
        users.add_user(User::new(1000, "enoch", 100));

        let user = f::User(1000);
        let expected = TextCell::paint_str(Blue.underline(), "enoch");
        assert_eq!(expected, user.render(&TestColours, &users));
    }

    #[test]
    fn different_unnamed_uid() {
        let user = f::User(1000);
        let expected = TextCell::paint_str(Blue.normal(), "1000");
        assert_eq!(
            expected,
            user.render_uid(&TestColours, &MockUsers::with_current_uid(0))
        );
    }

    #[test]
    fn overflow() {
        let user = f::User(2_147_483_648);
        let expected = TextCell::paint_str(Blue.normal(), "2147483648");
        assert_eq!(
            expected,
            user.render_uid(&TestColours, &MockUsers::with_current_uid(0))
        );
    }
}
