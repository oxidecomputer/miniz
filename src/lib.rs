/// Tiny in-memory implementation of the Zanzibar data model
///
/// See [`MiniZ`] for basic usage information.
///
/// A few notes:
///
/// * We use the term "relationship" where Zanzibar uses the term "relation" to
///   avoid confusion with the database term "relation".
/*
 * TODO:
 *
 * Things to implement:
 * - low-level operations:
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
 * - Add an example
 */
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

/// Unique id for a user-defined relationship
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RelationshipId(String);

#[derive(Debug)]
struct Relationship<O, U> {
    /// For a given object, set of Members (objects or users) having this
    /// relationship with that object
    ///
    /// This is changeable at runtime (after building).
    direct_members: BTreeMap<O, BTreeSet<Member<O, U>>>,

    /// Relationships that are implied by this relationship
    ///
    /// See [`RelationshipBuilder::with_subset`].
    contained_relationships: BTreeSet<RelationshipId>,

    /// Relationships that are inherited by this relationship
    ///
    /// This is similar to what Zanzibar calls "tuple_to_userset" when combined
    /// with "computed_userset".  
    ///
    /// A typical example: we say that the "viewer" relationship inherits the
    /// "parent" relationship, which means that if a user U has a "viewer"
    /// relationship to an object O1, and object O1 has the "parent"
    /// relationship to object O2, then U has a "viewer" relationship to O2 as
    /// well.  The implementation here would include "parent" in the set of
    /// inherited relationships for "viewer".
    ///
    /// Our implementation is less general than Zanzibar's, in that Zanzibar
    /// supports saying something like: if U has a "recursive-viewer"
    /// relationship with an object O1, and O1 is a parent of O2, then U has a
    /// "viewer" relationship with O2.  We require that the two relationships be
    /// the same.  This seems easy to generalize, though.
    ///
    inherited_relationships: BTreeSet<RelationshipId>,
}

///
/// Describes anything that can have a relationship to an Object
///
/// Confusingly, Zanzibar calls this a "user", and it may be either a user_id or
/// a "userset", and a "userset" is essentially an object-relation combination.
///
//
// XXX This seems like a divergence: S2.1 of the paper says that a tuple could
// be:
//
//     object_id # relation @ object_id # relation
//
// What does this mean?  I erroneusly assumed that this form meant:
//
//     object_id # relation @ object_id
//
// which would clearly mean the second object has the given relationship to the
// first object.
//
// I guess the first one means that the set of _users_ having relationship R2 to
// the second object have relationship R1 to the first object.  But if that's
// true: how do you express relationships between objects?  What if you want to
// say that O1 is a parent of O2?
//
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Member<O, U> {
    /// an object has the relationship to the given object
    Object(O),
    /// a user has the relationship to the given object
    User(U),
}

///
/// Describes one relationship that an object or user has
///
/// This is the dual of a [`Member`].
///
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Membership<O> {
    /// the object has relationship `rid`
    pub rid: RelationshipId,
    /// the relationship is to object `object`
    pub object: O,
}

///
/// Builder for a [`MiniZ`]
///
/// Conceptually, this object provides facilities that you might use when
/// processing a Zanzibar configuration file.  Anything that can be changed at
/// runtime (outside the configuration) does not belong here.
///
/// The only thing you can do with this is define relationships and then build a
/// [`MiniZ`].
///
#[derive(Debug)]
pub struct MiniZBuilder<O, U> {
    /// Configured relationships
    relationships: BTreeMap<RelationshipId, Relationship<O, U>>,
}

impl<O, U> MiniZBuilder<O, U>
where
    O: Clone + fmt::Debug + Ord,
    U: Clone + fmt::Debug + Ord,
{
    /// Defines a new `Relationship` named `relationship_name`
    pub fn new_relationship<S: AsRef<str>>(
        &mut self,
        relationship_name: S,
    ) -> RelationshipBuilder<'_, O, U> {
        RelationshipBuilder {
            miniz_builder: self,
            name: relationship_name.as_ref().to_owned(),
            contained_relationships: BTreeSet::new(),
            inherited_relationships: BTreeSet::new(),
        }
    }

    /// Returns a `MiniZ` with the configuration defined in the builder
    pub fn build(self) -> MiniZ<O, U> {
        MiniZ { sets: self.relationships, memberships: BTreeMap::new() }
    }
}

/// Used to configure a `Relationship`.  See [`MiniZBuilder`].
pub struct RelationshipBuilder<'a, O, U> {
    miniz_builder: &'a mut MiniZBuilder<O, U>,
    name: String,
    contained_relationships: BTreeSet<RelationshipId>,
    inherited_relationships: BTreeSet<RelationshipId>,
}

impl<'a, O, U> RelationshipBuilder<'a, O, U>
where
    O: Clone + fmt::Debug + Ord,
    U: Clone + fmt::Debug + Ord,
{
    ///
    /// Specify that the relationship `subrid` implies the current relationship
    ///
    /// A typical example: the "owner" relationship implies the "viewer"
    /// relationship, which means that any user that's an "owner" is also a
    /// "viewer".  More formally, if an object O1 has relationship `subrid` to
    /// another object O2, then it also has this relationship with O2.
    ///
    pub fn with_subset(mut self, subrid: &RelationshipId) -> Self {
        self.contained_relationships.insert(subrid.clone());
        self
    }

    ///
    /// Specify that the relationship `subrid` is inherited by the current
    /// relationship
    ///
    /// This is similar to what Zanzibar calls "tuple_to_userset" when combined
    /// with "computed_userset".  
    ///
    /// A typical example: we say that the "viewer" relationship inherits the
    /// "parent" relationship, which means that if a user U has a "viewer"
    /// relationship to an object O1, and object O1 has the "parent"
    /// relationship to object O2, then U has a "viewer" relationship to O2 as
    /// well.  The implementation here would include "parent" in the set of
    /// inherited relationships for "viewer".
    ///
    /// Our implementation is less general than Zanzibar's, in that Zanzibar
    /// supports saying something like: if U has a "recursive-viewer"
    /// relationship with an object O1, and O1 is a parent of O2, then U has a
    /// "viewer" relationship with O2.  Unlike Zanzibar, we require that the two
    /// relationships be the same.  (This seems easy to generalize, though.)
    ///
    pub fn with_inherited_set(mut self, rid: &RelationshipId) -> Self {
        self.inherited_relationships.insert(rid.clone());
        self
    }

    ///
    /// Add the relationship configured by this builder to the parent
    /// [`MiniZBuilder`] and return a [`RelationshipId`] for it.
    ///
    pub fn build(self) -> RelationshipId {
        let rid = RelationshipId(self.name);
        self.miniz_builder.relationships.insert(
            rid.clone(),
            Relationship {
                direct_members: BTreeMap::new(),
                contained_relationships: self.contained_relationships,
                inherited_relationships: self.inherited_relationships,
            },
        );

        rid
    }
}

///
/// A toy in-memory implementation of the Zanzibar data model
///
/// In the Zanzibar data model, consumers statically configure relationships
/// (like "parent" or "viewer").  At runtime, consumers add or remove specific
/// relationships (e.g., X is a "parent" of Y).  In this API, the static
/// configuration is provided via the [`MiniZBuilder`] (see
/// [`MiniZ::builder()`]).  The runtime operations are provided by [`MiniZ`].
///
/// See the test case below for an example that corresponds to the one in the
/// Zanzibar paper.
///
pub struct MiniZ<O, U> {
    sets: BTreeMap<RelationshipId, Relationship<O, U>>,
    memberships: BTreeMap<Member<O, U>, BTreeSet<Membership<O>>>,
}

impl<O, U> MiniZ<O, U>
where
    O: Clone + fmt::Debug + Ord,
    U: Clone + fmt::Debug + Ord,
{
    /// Return a builder used to configure relationships known to this instance
    pub fn builder() -> MiniZBuilder<O, U> {
        MiniZBuilder { relationships: BTreeMap::new() }
    }

    /*
     * Write operations
     */

    ///
    /// Specify that object `child` directly has the `rid` relationship to
    /// object `parent`
    ///
    /// (The relationship need not be hierarchical like "parent" is, but it's
    /// easier to talk about the two objects with concrete names.)
    ///
    pub fn write_object(&mut self, rid: &RelationshipId, parent: O, child: O) {
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

    ///
    /// Specify that user `child` directly has the `rid` relationship to object
    /// `parent`
    ///
    /// (The relationship need not be hierarchical like "parent" is, but it's
    /// easier to talk about the two objects with concrete names.)
    ///
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

    ///
    /// Returns whether object `child` has relationship `rid` with object
    /// `parent` _directly_.  To check for indirect relationships, see
    /// [`check_member()`].
    ///
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

    ///
    /// Returns whether user `child` has relationship `rid` with object
    /// `parent` _directly_.  To check for indirect relationships, see
    /// [`check_member()`].
    ///
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

    /// List the users and objects having a direct relationship with `parent`
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

    /// List the objects that this object has a direct relationship with
    pub fn object_lookup_memberships(&self, object: O) -> Vec<&Membership<O>> {
        match self.memberships.get(&Member::Object(object)) {
            Some(memberships) => memberships.iter().collect(),
            None => Vec::new(),
        }
    }

    /// List the objects that this user has a direct relationship with
    pub fn user_lookup_memberships(&self, user: U) -> Vec<&Membership<O>> {
        match self.memberships.get(&Member::User(user)) {
            Some(memberships) => memberships.iter().collect(),
            None => Vec::new(),
        }
    }

    ///
    /// Check whether the user `user` has relationship `rid` with object
    /// `object`, either directly or through a combination of implied or
    /// inherited relationships
    ///
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
        for subrid in &set.contained_relationships {
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
        let memberships = self.memberships.get(&Member::Object(object));
        if memberships.is_none() {
            return false;
        }

        let inherited_present_memberships = memberships
            .unwrap()
            .into_iter()
            .filter(|m| set.inherited_relationships.contains(&m.rid));
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
        let set_owner = miniz_builder.new_relationship("owner").build();
        let set_parent = miniz_builder.new_relationship("parent").build();
        let set_editor = miniz_builder
            .new_relationship("editor")
            .with_subset(&set_owner)
            .build();
        let set_viewer = miniz_builder
            .new_relationship("viewer")
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
            &dir1,
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
