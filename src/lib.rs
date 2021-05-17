/// Tiny in-memory implementation of the Zanzibar data model
/*
 * TODO:
 *
 * For clarity:
 * * "Sets" are really "Relationships".  Each one really defines a _family_ of
 *   sets -- one for each possible objectid.
 *
 * Things to implement:
 * - low-level operations:
 *   - Check membership including subsets and inherited sets.  This is
 *     really the key.
 *   - Remove member from set (needed to flesh out "write")
 *   - low level operations needed for "expand"
 * - higher level operations from section 2.4 of the paper
 *   - "Check": checks membership, including subsets and inherited sets
 *   - "Read": a bit more flexible than what I have here, but the gist is
 *     here
 *   - "Write": excuding OCC, this is (presumably) the add/remove operations
 *     we already have here
 *   - "Expand"
 *
 * General:
 * - Decide if the ID types ought to just be Copy, or if we should create
 *   our own IDs via interning them or what.  Either way, we shouldn't be
 *   cloning all over the place.
 */
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

/// Unique id for a user-defined relationship
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RelationshipId(String);

#[derive(Debug)]
struct Set<O, U> {
    direct_members: BTreeMap<O, BTreeSet<Member<O, U>>>,
    contained_sets: BTreeSet<RelationshipId>,
    inherited_sets: BTreeSet<RelationshipId>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Member<O, U> {
    Object(O),
    User(U),
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Membership<O> {
    pub rid: RelationshipId,
    pub object: O,
}

#[derive(Debug)]
pub struct MiniZBuilder<O, U> {
    sets: BTreeMap<RelationshipId, Set<O, U>>,
}

impl<O, U> MiniZBuilder<O, U>
where
    O: Clone + fmt::Debug + Ord,
    U: Clone + fmt::Debug + Ord,
{
    pub fn new_set<S: AsRef<str>>(
        &mut self,
        set_name: S,
    ) -> SetBuilder<'_, O, U> {
        SetBuilder {
            miniz_builder: self,
            name: set_name.as_ref().to_owned(),
            contained_sets: BTreeSet::new(),
            inherited_sets: BTreeSet::new(),
        }
    }

    pub fn build(self) -> MiniZ<O, U> {
        MiniZ { sets: self.sets, memberships: BTreeMap::new() }
    }
}

pub struct SetBuilder<'a, O, U> {
    miniz_builder: &'a mut MiniZBuilder<O, U>,
    name: String,
    contained_sets: BTreeSet<RelationshipId>,
    inherited_sets: BTreeSet<RelationshipId>,
}

impl<'a, O, U> SetBuilder<'a, O, U>
where
    O: Clone + fmt::Debug + Ord,
    U: Clone + fmt::Debug + Ord,
{
    pub fn with_subset(mut self, subrid: &RelationshipId) -> Self {
        self.contained_sets.insert(subrid.clone());
        self
    }

    pub fn with_inherited_set(mut self, rid: &RelationshipId) -> Self {
        self.inherited_sets.insert(rid.clone());
        self
    }

    pub fn build(self) -> RelationshipId {
        let rid = RelationshipId(self.name);
        self.miniz_builder.sets.insert(
            rid.clone(),
            Set {
                direct_members: BTreeMap::new(),
                contained_sets: self.contained_sets,
                inherited_sets: self.inherited_sets,
            },
        );

        rid
    }
}

pub struct MiniZ<O, U> {
    sets: BTreeMap<RelationshipId, Set<O, U>>,
    memberships: BTreeMap<Member<O, U>, BTreeSet<Membership<O>>>,
}

impl<O, U> MiniZ<O, U>
where
    O: Clone + fmt::Debug + Ord,
    U: Clone + fmt::Debug + Ord,
{
    pub fn builder() -> MiniZBuilder<O, U> {
        MiniZBuilder { sets: BTreeMap::new() }
    }

    /*
     * Write operations
     */

    pub fn write_object(
        &mut self,
        rid: &RelationshipId,
        parent: O,
        child: O,
    ) {
        let set = self.sets.get_mut(rid).expect("no such set");
        let members = set
            .direct_members
            .entry(parent.clone())
            .or_insert_with(BTreeSet::new);
        let new_value = Member::Object(child);
        assert!(!members.contains(&new_value));
        assert!(members.insert(new_value.clone()));

        /* Update the reverse index. */
        let memberships =
            self.memberships.entry(new_value).or_insert_with(BTreeSet::new);
        memberships.insert(Membership { rid: rid.clone(), object: parent });
    }

    pub fn write_user(&mut self, rid: &RelationshipId, parent: O, child: U) {
        let set = self.sets.get_mut(rid).expect("no such set");
        let members = set
            .direct_members
            .entry(parent.clone())
            .or_insert_with(BTreeSet::new);
        let new_value = Member::User(child);
        assert!(!members.contains(&new_value));
        assert!(members.insert(new_value.clone()));

        /* Update the reverse index. */
        let memberships =
            self.memberships.entry(new_value).or_insert_with(BTreeSet::new);
        memberships.insert(Membership { rid: rid.clone(), object: parent });
    }

    /*
     * Read operations
     */

    pub fn set_contains_object_directly(
        &self,
        rid: &RelationshipId,
        parent: &O,
        child: O,
    ) -> bool {
        let set = self.sets.get(rid).expect("no such set");
        match set.direct_members.get(parent) {
            Some(members) => members.contains(&Member::Object(child)),
            None => false,
        }
    }

    pub fn set_contains_user_directly(
        &self,
        rid: &RelationshipId,
        parent: &O,
        child: U,
    ) -> bool {
        let set = self.sets.get(rid).expect("no such set");
        match set.direct_members.get(parent) {
            Some(members) => members.contains(&Member::User(child)),
            None => false,
        }
    }

    pub fn set_list_direct_members<'a, 'b>(
        &'a self,
        rid: &'b RelationshipId,
        parent: &'b O,
    ) -> Vec<&Member<O, U>> {
        let set = self.sets.get(rid).expect("no such set");
        match set.direct_members.get(parent) {
            Some(members) => members.iter().collect(),
            None => Vec::new(),
        }
    }

    pub fn object_lookup_memberships(&self, object: O) -> Vec<&Membership<O>> {
        match self.memberships.get(&Member::Object(object)) {
            Some(memberships) => memberships.iter().collect(),
            None => Vec::new(),
        }
    }

    pub fn user_lookup_memberships(&self, user: U) -> Vec<&Membership<O>> {
        match self.memberships.get(&Member::User(user)) {
            Some(memberships) => memberships.iter().collect(),
            None => Vec::new(),
        }
    }

    pub fn check_member(
        &self,
        rid: &RelationshipId,
        object: O,
        user: U,
    ) -> bool {
        let set = self.sets.get(rid).expect("no such set");

        /*
         * First, check if the user is a direct member of this set.
         */
        if let Some(members) = set.direct_members.get(&object) {
            if members.contains(&Member::User(user.clone())) {
                return true;
            }
        }

        /*
         * Next, check recursively if the user is a member (directly or
         * otherwise) of a set directly contained in this set.
         */
        for subrid in &set.contained_sets {
            if self.check_member(subrid, object.clone(), user.clone()) {
                return true;
            }
        }

        /*
         * This is more expensive.  Check if there exists any object O2 such
         * that the user has the desired relationship with O2 and this object
         * inherits O2's relationships.
         * XXX This could be more efficient with another index.
         */
        let memberships = self.memberships.get(&Member::Object(object.clone()));
        if memberships.is_none() {
            return false;
        }

        let inherited_present_memberships = memberships
            .unwrap()
            .into_iter()
            .filter(|m| set.inherited_sets.contains(&m.rid));
        for m in inherited_present_memberships {
            if self.check_member(&m.rid, m.object.clone(), user.clone()) {
                return true;
            }
        }

        return false;
    }
}

#[cfg(test)]
mod test {
    use super::Member;
    use super::Membership;
    use super::MiniZ;

    #[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
    struct ObjectId(&'static str);
    #[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
    struct UserId(&'static str);

    #[test]
    fn test_example() {
        /*
         * The following block constructs a MiniZ instance with the same
         * configuration as what's described in Figure 1 in the Zanzibar paper.
         * It looks intended to describe the Google Docs authorization behavior.
         */
        let mut miniz_builder = MiniZ::builder();
        let set_owner = miniz_builder.new_set("owner").build();
        let set_parent = miniz_builder.new_set("parent").build();
        let set_editor =
            miniz_builder.new_set("editor").with_subset(&set_owner).build();
        let set_viewer = miniz_builder
            .new_set("viewer")
            .with_subset(&set_editor)
            .with_inherited_set(&set_parent)
            .build();
        let mut miniz = miniz_builder.build();

        /*
         * Now demo it.  We'll construct this hierarchy of objects (defined by
         * the "parent" relationship).
         *
         *    "dir1"                  owner: "alice"
         *      |                     editor: "bob"
         *      | "parent"            viewer: "carol"
         *      v
         *    "doc123"                owner: "dan"
         *                            editor: "eve"
         *                            viewer: "faythe"
         *
         *
         *    "dir2"                  owner: "heidi"
         *      |                     editor: "ivan"
         *      |                     viewer: "judy"
         *      v
         *    "doc456"                owner: "oscar"
         *
         * (The names here are taken from
         * https://en.wikipedia.org/wiki/Alice_and_Bob#Cast_of_characters.)
         *
         * TODO could add another parent directory of "dir1" to test recursion.
         * TODO could add more levels of indirection, too?
         */
        let dir1 = ObjectId("dir1");
        let dir2 = ObjectId("dir2");
        let dir3 = ObjectId("dir3");
        let doc123 = ObjectId("doc123");
        let doc456 = ObjectId("doc456");
        let doc789 = ObjectId("doc789");

        let user_alice = UserId("alice");
        let user_bob = UserId("bob");
        let user_carol = UserId("carol");
        let user_dan = UserId("dan");
        let user_eve = UserId("eve");
        let user_faythe = UserId("faythe");
        let user_heidi = UserId("heidi");
        let user_ivan = UserId("ivan");
        let user_judy = UserId("judy");
        let user_oscar = UserId("oscar");

        miniz.write_object(&set_parent, dir1, doc123);
        miniz.write_user(&set_owner, dir1, user_alice);
        miniz.write_user(&set_editor, dir1, user_bob);
        miniz.write_user(&set_viewer, dir1, user_carol);

        miniz.write_user(&set_owner, doc123, user_dan);
        miniz.write_user(&set_editor, doc123, user_eve);
        miniz.write_user(&set_viewer, doc123, user_faythe);

        miniz.write_object(&set_parent, dir2, doc456);
        miniz.write_user(&set_owner, dir2, user_heidi);
        miniz.write_user(&set_editor, dir2, user_ivan);
        miniz.write_user(&set_viewer, dir2, user_judy);
        miniz.write_user(&set_owner, doc456, user_oscar);

        /* Contents of "dir1" */
        assert!(miniz.set_contains_object_directly(&set_parent, &dir1, doc123));
        assert!(!miniz.set_contains_object_directly(
            &set_parent,
            &dir1,
            doc456
        ));
        /* Contents of "dir2" */
        assert!(miniz.set_contains_object_directly(&set_parent, &dir2, doc456));
        assert!(!miniz.set_contains_object_directly(
            &set_parent,
            &dir2,
            doc123
        ));
        /* Non-existent document is contained nowhere. */
        assert!(!miniz.set_contains_object_directly(
            &set_parent,
            &dir2,
            doc789
        ));
        assert!(!miniz.set_contains_object_directly(
            &set_parent,
            &dir2,
            doc789
        ));
        /* Non-existent set contains nothing */
        assert!(!miniz.set_contains_object_directly(
            &set_parent,
            &dir3,
            doc123
        ));
        assert!(!miniz.set_contains_object_directly(
            &set_parent,
            &dir3,
            doc123
        ));

        /* User associations (direct associations) for "dir1" */
        assert_eq!(
            miniz.set_list_direct_members(&set_owner, &dir1),
            vec![&Member::User(user_alice)]
        );
        assert!(miniz.set_contains_user_directly(&set_owner, &dir1, user_alice));
        assert!(!miniz.set_contains_user_directly(&set_owner, &dir1, user_bob));
        assert!(
            !miniz.set_contains_user_directly(&set_owner, &dir1, user_carol)
        );

        assert_eq!(
            miniz.set_list_direct_members(&set_editor, &dir1),
            vec![&Member::User(user_bob)]
        );
        assert!(!miniz.set_contains_user_directly(
            &set_editor,
            &dir1,
            user_alice
        ));
        assert!(miniz.set_contains_user_directly(&set_editor, &dir1, user_bob));
        assert!(!miniz.set_contains_user_directly(
            &set_editor,
            &dir1,
            user_carol
        ));

        assert_eq!(
            miniz.set_list_direct_members(&set_viewer, &dir1),
            vec![&Member::User(user_carol)]
        );
        assert!(!miniz.set_contains_user_directly(
            &set_viewer,
            &dir1,
            user_alice
        ));
        assert!(!miniz.set_contains_user_directly(
            &set_viewer,
            &dir1,
            user_bob
        ));
        assert!(miniz.set_contains_user_directly(
            &set_viewer,
            &dir1,
            user_carol
        ));

        /* Reverse indexes */
        assert_eq!(
            miniz.object_lookup_memberships(dir1),
            Vec::new() as Vec<&Membership<ObjectId>>,
        );
        assert_eq!(
            miniz.object_lookup_memberships(dir2),
            Vec::new() as Vec<&Membership<ObjectId>>,
        );
        assert_eq!(
            miniz.object_lookup_memberships(doc123),
            vec![&Membership { rid: set_parent.clone(), object: dir1 }]
        );
        assert_eq!(
            miniz.user_lookup_memberships(user_alice),
            vec![&Membership { rid: set_owner.clone(), object: dir1 }]
        );

        /* "Check" API */
        assert!(miniz.check_member(&set_viewer, dir1, user_alice));
        assert!(miniz.check_member(&set_viewer, dir1, user_bob));
        assert!(miniz.check_member(&set_viewer, dir1, user_carol));
        assert!(miniz.check_member(&set_editor, dir1, user_alice));
        assert!(miniz.check_member(&set_editor, dir1, user_bob));
        assert!(!miniz.check_member(&set_editor, dir1, user_carol));
        assert!(miniz.check_member(&set_owner, dir1, user_alice));
        assert!(!miniz.check_member(&set_owner, dir1, user_bob));
        assert!(!miniz.check_member(&set_owner, dir1, user_carol));
    }
}
