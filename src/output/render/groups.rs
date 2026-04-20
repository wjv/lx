use std::collections::HashSet;
use std::sync::LazyLock;

use nu_ansi_term::Style;
use uzers::{Groups, Users};

use crate::fs::fields as f;
use crate::output::cell::TextCell;

/// The current process's full group set: effective GID + supplementary
/// groups.  Cached for the lifetime of the process.
static MY_GROUPS: LazyLock<HashSet<u32>> = LazyLock::new(|| {
    let mut groups = HashSet::new();
    // Effective GID (primary group).
    groups.insert(uzers::get_effective_gid());
    // Supplementary groups (on Linux this includes the egid;
    // on other platforms it may not — the insert above covers it).
    if let Some(list) = uzers::get_user_groups(
        &uzers::get_current_username().unwrap_or_default(),
        uzers::get_effective_gid(),
    ) {
        for g in &list {
            groups.insert(g.gid());
        }
    }
    groups
});

impl f::Group {
    /// Render the group as its name, falling back to the numeric GID
    /// if the name can't be resolved.  Used for the `--group` column.
    pub fn render<C: Colours, U: Users + Groups>(self, colours: &C, users: &U) -> TextCell {
        let style = self.style(colours, users, /* gid_column= */ false);
        match self.lookup_name(users) {
            Some(name) => TextCell::paint(style, name),
            None => TextCell::paint(style, self.0.to_string()),
        }
    }

    /// Render the group as the raw numeric GID.  Used for the `--gid`
    /// column.  Uses the dedicated `gid_*` style slots so themes can
    /// distinguish it visually from the name column while still
    /// cascading from the `group_*` slots when unset.
    pub fn render_gid<C: Colours, U: Users + Groups>(self, colours: &C, users: &U) -> TextCell {
        TextCell::paint(
            self.style(colours, users, /* gid_column= */ true),
            self.0.to_string(),
        )
    }

    fn lookup_name<U: Users + Groups>(self, users: &U) -> Option<String> {
        users
            .get_group_by_gid(self.0)
            .map(|g| g.name().to_string_lossy().into())
    }

    /// Compute the style for this group.  Three tiers: primary group,
    /// supplementary group (you have group access), or other.
    /// Uses `getgroups()` (cached in `MY_GROUPS`) rather than
    /// `/etc/group` membership lists, so macOS Directory Services
    /// groups and LDAP groups are handled correctly.
    /// `gid_column` selects the `gid_*` style slots.
    fn style<C: Colours, U: Users + Groups>(
        self,
        colours: &C,
        _users: &U,
        gid_column: bool,
    ) -> Style {
        let tier = if self.0 == uzers::get_effective_gid() {
            GroupTier::Primary
        } else if MY_GROUPS.contains(&self.0) {
            GroupTier::Member
        } else {
            GroupTier::Other
        };

        match (gid_column, tier) {
            (false, GroupTier::Primary) => colours.yours(),
            (false, GroupTier::Member) => colours.member(),
            (false, GroupTier::Other) => colours.not_yours(),
            (true, GroupTier::Primary) => colours.gid_yours(),
            (true, GroupTier::Member) => colours.gid_member(),
            (true, GroupTier::Other) => colours.gid_not_yours(),
        }
    }
}

enum GroupTier {
    Primary,
    Member,
    Other,
}

pub trait Colours {
    fn yours(&self) -> Style;
    fn member(&self) -> Style;
    fn not_yours(&self) -> Style;

    fn gid_yours(&self) -> Style;
    fn gid_member(&self) -> Style;
    fn gid_not_yours(&self) -> Style;
}

#[cfg(test)]
#[allow(unused_results)]
pub mod test {
    use super::Colours;
    use crate::fs::fields as f;
    use crate::output::cell::TextCell;

    use nu_ansi_term::Color::*;
    use nu_ansi_term::Style;
    use uzers::Group;
    use uzers::mock::MockUsers;

    /// Test colours with distinct slots for name/GID columns so
    /// tests can verify `render_gid` picks up the GID-specific
    /// styles.  Real cascade from group slots is covered by
    /// theme-level tests.
    struct TestColours;

    impl Colours for TestColours {
        fn yours(&self) -> Style {
            Fixed(80).normal()
        }
        fn member(&self) -> Style {
            Fixed(84).normal()
        }
        fn not_yours(&self) -> Style {
            Fixed(81).normal()
        }
        fn gid_yours(&self) -> Style {
            Fixed(82).normal()
        }
        fn gid_member(&self) -> Style {
            Fixed(85).normal()
        }
        fn gid_not_yours(&self) -> Style {
            Fixed(83).normal()
        }
    }

    /// Use a GID that's guaranteed not in the test runner's real
    /// supplementary group set.
    const FOREIGN_GID: u32 = 99999;

    #[test]
    fn named_other_group() {
        let mut users = MockUsers::with_current_uid(1000);
        users.add_group(Group::new(FOREIGN_GID, "folk"));

        let group = f::Group(FOREIGN_GID);
        let expected = TextCell::paint_str(Fixed(81).normal(), "folk");
        assert_eq!(expected, group.render(&TestColours, &users));

        let expected = TextCell::paint(Fixed(83).normal(), FOREIGN_GID.to_string());
        assert_eq!(expected, group.render_gid(&TestColours, &users));
    }

    #[test]
    fn unnamed_falls_back_to_gid() {
        let users = MockUsers::with_current_uid(1000);

        let group = f::Group(FOREIGN_GID);
        let expected_name = TextCell::paint(Fixed(81).normal(), FOREIGN_GID.to_string());
        assert_eq!(expected_name, group.render(&TestColours, &users));
        let expected_gid = TextCell::paint(Fixed(83).normal(), FOREIGN_GID.to_string());
        assert_eq!(expected_gid, group.render_gid(&TestColours, &users));
    }

    #[test]
    fn primary_group() {
        // Use the real effective GID so MY_GROUPS recognises it.
        let egid = uzers::get_effective_gid();
        let mut users = MockUsers::with_current_uid(uzers::get_current_uid());
        users.add_group(Group::new(egid, "primary"));

        let group = f::Group(egid);
        let expected = TextCell::paint_str(Fixed(80).normal(), "primary");
        assert_eq!(expected, group.render(&TestColours, &users));
    }

    #[test]
    fn member_group() {
        // Pick a supplementary group that isn't the primary.
        let egid = uzers::get_effective_gid();
        let member_gid = super::MY_GROUPS.iter().find(|&&g| g != egid).copied();

        if let Some(gid) = member_gid {
            let mut users = MockUsers::with_current_uid(uzers::get_current_uid());
            users.add_group(Group::new(gid, "supplementary"));

            let group = f::Group(gid);
            let expected = TextCell::paint_str(Fixed(84).normal(), "supplementary");
            assert_eq!(expected, group.render(&TestColours, &users));
        }
        // Skip if the test runner has no supplementary groups.
    }

    #[test]
    fn overflow() {
        let group = f::Group(2_147_483_648);
        let expected = TextCell::paint_str(Fixed(83).normal(), "2147483648");
        assert_eq!(
            expected,
            group.render_gid(&TestColours, &MockUsers::with_current_uid(0))
        );
    }
}
