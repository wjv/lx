use nu_ansi_term::Style;
use uzers::Users;

use crate::fs::fields as f;
use crate::output::cell::TextCell;
use crate::theme::Theme;

impl f::User {
    /// Render the owner as their name, falling back to the numeric
    /// UID if the name can't be resolved.  Used for the `--user` column.
    pub fn render<U: Users>(self, theme: &Theme, users: &U) -> TextCell {
        let user_name = match users.get_user_by_uid(self.0) {
            None => self.0.to_string(),
            Some(user) => user.name().to_string_lossy().into(),
        };

        TextCell::paint(self.style(theme, users), user_name)
    }

    /// Render the owner as the raw numeric UID.  Used for the
    /// `--uid` column.  Uses the dedicated `uid_*` style slots so
    /// themes can distinguish it visually from the name column.
    pub fn render_uid<U: Users>(self, theme: &Theme, users: &U) -> TextCell {
        TextCell::paint(self.uid_style(theme, users), self.0.to_string())
    }

    fn style<U: Users>(self, theme: &Theme, users: &U) -> Style {
        if users.get_current_uid() == self.0 {
            theme.ui.users.user_you
        } else {
            theme.ui.users.user_someone_else
        }
    }

    fn uid_style<U: Users>(self, theme: &Theme, users: &U) -> Style {
        if users.get_current_uid() == self.0 {
            theme.ui.users.uid_you
        } else {
            theme.ui.users.uid_someone_else
        }
    }
}

#[cfg(test)]
#[allow(unused_results)]
pub mod test {
    use crate::fs::fields as f;
    use crate::output::cell::TextCell;
    use crate::theme::Theme;

    use nu_ansi_term::Color::*;
    use uzers::User;
    use uzers::mock::MockUsers;

    /// Cascading test theme: the `uid_*` slots use slightly
    /// different styles than the `user_*` slots, so the tests can
    /// verify that `render_uid` picks up the UID slots.
    fn theme() -> Theme {
        let mut t = Theme::test_default();
        t.ui.users.user_you = Red.bold();
        t.ui.users.user_someone_else = Blue.underline();
        t.ui.users.uid_you = Red.normal();
        t.ui.users.uid_someone_else = Blue.normal();
        t
    }

    #[test]
    fn named() {
        let mut users = MockUsers::with_current_uid(1000);
        users.add_user(User::new(1000, "enoch", 100));

        let user = f::User(1000);
        let expected = TextCell::paint_str(Red.bold(), "enoch");
        assert_eq!(expected, user.render(&theme(), &users));

        let expected = TextCell::paint_str(Red.normal(), "1000");
        assert_eq!(expected, user.render_uid(&theme(), &users));
    }

    #[test]
    fn unnamed_falls_back_to_uid() {
        let users = MockUsers::with_current_uid(1000);

        let user = f::User(1000);
        let expected_name = TextCell::paint_str(Red.bold(), "1000");
        assert_eq!(expected_name, user.render(&theme(), &users));
        let expected_uid = TextCell::paint_str(Red.normal(), "1000");
        assert_eq!(expected_uid, user.render_uid(&theme(), &users));
    }

    #[test]
    fn different_named() {
        let mut users = MockUsers::with_current_uid(0);
        users.add_user(User::new(1000, "enoch", 100));

        let user = f::User(1000);
        let expected = TextCell::paint_str(Blue.underline(), "enoch");
        assert_eq!(expected, user.render(&theme(), &users));
    }

    #[test]
    fn different_unnamed_uid() {
        let user = f::User(1000);
        let expected = TextCell::paint_str(Blue.normal(), "1000");
        assert_eq!(
            expected,
            user.render_uid(&theme(), &MockUsers::with_current_uid(0))
        );
    }

    #[test]
    fn overflow() {
        let user = f::User(2_147_483_648);
        let expected = TextCell::paint_str(Blue.normal(), "2147483648");
        assert_eq!(
            expected,
            user.render_uid(&theme(), &MockUsers::with_current_uid(0))
        );
    }
}
