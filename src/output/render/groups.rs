use nu_ansi_term::Style;
use uzers::{Users, Groups};

use crate::fs::fields as f;
use crate::output::cell::TextCell;


impl f::Group {
    /// Render the group as its name, falling back to the numeric GID
    /// if the name can't be resolved.  Used for the `--group` column.
    pub fn render<C: Colours, U: Users+Groups>(self, colours: &C, users: &U) -> TextCell {
        let style = self.style(colours, users, /* gid_column= */ false);
        match self.lookup_name(users) {
            Some(name) => TextCell::paint(style, name),
            None       => TextCell::paint(style, self.0.to_string()),
        }
    }

    /// Render the group as the raw numeric GID.  Used for the `--gid`
    /// column.  Uses the dedicated `gid_*` style slots so themes can
    /// distinguish it visually from the name column while still
    /// cascading from the `group_*` slots when unset.
    pub fn render_gid<C: Colours, U: Users+Groups>(self, colours: &C, users: &U) -> TextCell {
        TextCell::paint(self.style(colours, users, /* gid_column= */ true), self.0.to_string())
    }

    fn lookup_name<U: Users+Groups>(self, users: &U) -> Option<String> {
        users.get_group_by_gid(self.0)
            .map(|g| g.name().to_string_lossy().into())
    }

    /// Compute the style for this group based on whether the current
    /// user is a member (primary or secondary).  `gid_column` selects
    /// the dedicated `gid_*` style slots; the name column uses the
    /// `group_*` slots.
    fn style<C: Colours, U: Users+Groups>(self, colours: &C, users: &U, gid_column: bool) -> Style {
        use uzers::os::unix::GroupExt;

        let is_member = users.get_group_by_gid(self.0).is_some_and(|group| {
            let group = (*group).clone();
            let current_uid = users.get_current_uid();
            users.get_user_by_uid(current_uid).is_some_and(|current_user| {
                current_user.primary_group_id() == group.gid()
                    || group.members().iter().any(|u| u == current_user.name())
            })
        });

        match (gid_column, is_member) {
            (false, true)  => colours.yours(),
            (false, false) => colours.not_yours(),
            (true,  true)  => colours.gid_yours(),
            (true,  false) => colours.gid_not_yours(),
        }
    }
}


pub trait Colours {
    fn yours(&self) -> Style;
    fn not_yours(&self) -> Style;

    /// Style for the numeric GID column, for groups the current
    /// user is a member of.  Cascades from `yours()` when unset.
    fn gid_yours(&self) -> Style;

    /// Style for the numeric GID column, for other groups.
    /// Cascades from `not_yours()` when unset.
    fn gid_not_yours(&self) -> Style;
}


#[cfg(test)]
#[allow(unused_results)]
pub mod test {
    use super::Colours;
    use crate::fs::fields as f;
    use crate::output::cell::TextCell;

    use uzers::{User, Group};
    use uzers::mock::MockUsers;
    use uzers::os::unix::GroupExt;
    use nu_ansi_term::Color::*;
    use nu_ansi_term::Style;


    /// Test colours with distinct slots for name/GID columns so
    /// tests can verify `render_gid` picks up the GID-specific
    /// styles.  Real cascade from group slots is covered by
    /// theme-level tests.
    struct TestColours;

    impl Colours for TestColours {
        fn yours(&self)         -> Style { Fixed(80).normal() }
        fn not_yours(&self)     -> Style { Fixed(81).normal() }
        fn gid_yours(&self)     -> Style { Fixed(82).normal() }
        fn gid_not_yours(&self) -> Style { Fixed(83).normal() }
    }


    #[test]
    fn named() {
        let mut users = MockUsers::with_current_uid(1000);
        users.add_group(Group::new(100, "folk"));

        let group = f::Group(100);
        let expected = TextCell::paint_str(Fixed(81).normal(), "folk");
        assert_eq!(expected, group.render(&TestColours, &users));

        // GID column uses its dedicated slot.
        let expected = TextCell::paint_str(Fixed(83).normal(), "100");
        assert_eq!(expected, group.render_gid(&TestColours, &users));
    }


    #[test]
    fn unnamed_falls_back_to_gid() {
        let users = MockUsers::with_current_uid(1000);

        let group = f::Group(100);
        // Name column falls back to numeric GID but keeps the group slot.
        let expected_name = TextCell::paint_str(Fixed(81).normal(), "100");
        assert_eq!(expected_name, group.render(&TestColours, &users));
        // GID column always uses its dedicated slot.
        let expected_gid = TextCell::paint_str(Fixed(83).normal(), "100");
        assert_eq!(expected_gid, group.render_gid(&TestColours, &users));
    }

    #[test]
    fn primary() {
        let mut users = MockUsers::with_current_uid(2);
        users.add_user(User::new(2, "eve", 100));
        users.add_group(Group::new(100, "folk"));

        let group = f::Group(100);
        let expected = TextCell::paint_str(Fixed(80).normal(), "folk");
        assert_eq!(expected, group.render(&TestColours, &users))
    }

    #[test]
    fn secondary() {
        let mut users = MockUsers::with_current_uid(2);
        users.add_user(User::new(2, "eve", 666));

        let test_group = Group::new(100, "folk").add_member("eve");
        users.add_group(test_group);

        let group = f::Group(100);
        let expected = TextCell::paint_str(Fixed(80).normal(), "folk");
        assert_eq!(expected, group.render(&TestColours, &users))
    }

    #[test]
    fn overflow() {
        let group = f::Group(2_147_483_648);
        // render_gid on a not-yours group uses gid_not_yours slot.
        let expected = TextCell::paint_str(Fixed(83).normal(), "2147483648");
        assert_eq!(expected, group.render_gid(&TestColours, &MockUsers::with_current_uid(0)));
    }
}
