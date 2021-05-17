use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SetId(String);

#[derive(Debug)]
pub struct Set<O, U> {
    direct_members: BTreeMap<O, BTreeSet<Member<O, U>>>,
    contained_sets: BTreeSet<SetId>,
    inherited_sets: BTreeSet<SetId>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Member<O, U> {
    Object(O),
    User(U),
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Membership<O> {
    pub set_id: SetId,
    pub object: O,
}

#[derive(Debug)]
pub struct MiniZBuilder<O, U> {
    sets: BTreeMap<SetId, Set<O, U>>,
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
    contained_sets: BTreeSet<SetId>,
    inherited_sets: BTreeSet<SetId>,
}

impl<'a, O, U> SetBuilder<'a, O, U>
where
    O: Clone + fmt::Debug + Ord,
    U: Clone + fmt::Debug + Ord,
{
    pub fn with_subset(mut self, subset_id: &SetId) -> Self {
        self.contained_sets.insert(subset_id.clone());
        self
    }

    pub fn with_inherited_set(mut self, set_id: &SetId) -> Self {
        self.inherited_sets.insert(set_id.clone());
        self
    }

    pub fn build(self) -> SetId {
        let set_id = SetId(self.name);
        self.miniz_builder.sets.insert(
            set_id.clone(),
            Set {
                direct_members: BTreeMap::new(),
                contained_sets: self.contained_sets,
                inherited_sets: self.inherited_sets,
            },
        );

        set_id
    }
}

pub struct MiniZ<O, U> {
    sets: BTreeMap<SetId, Set<O, U>>,
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

    pub fn write_object(&mut self, set_id: &SetId, parent: O, child: O) {
        let set = self.sets.get_mut(set_id).expect("no such set");
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
        memberships
            .insert(Membership { set_id: set_id.clone(), object: parent });
    }

    pub fn write_user(&mut self, set_id: &SetId, parent: O, child: U) {
        let set = self.sets.get_mut(set_id).expect("no such set");
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
        memberships
            .insert(Membership { set_id: set_id.clone(), object: parent });
    }

    /*
     * Read operations
     */

    /* TODO-cleanup should not need to consume "child" */
    pub fn set_contains_object_directly(
        &self,
        set_id: &SetId,
        parent: &O,
        child: O,
    ) -> bool {
        let set = self.sets.get(set_id).expect("no such set");
        match set.direct_members.get(parent) {
            Some(members) => members.contains(&Member::Object(child)),
            None => false,
        }
    }

    /* TODO-cleanup should not need to consume "child" */
    pub fn set_contains_user_directly(
        &self,
        set_id: &SetId,
        parent: &O,
        child: U,
    ) -> bool {
        let set = self.sets.get(set_id).expect("no such set");
        match set.direct_members.get(parent) {
            Some(members) => members.contains(&Member::User(child)),
            None => false,
        }
    }

    pub fn set_list_direct_members<'a, 'b>(
        &'a self,
        set_id: &'b SetId,
        parent: &'b O,
    ) -> Vec<&Member<O, U>> {
        let set = self.sets.get(set_id).expect("no such set");
        match set.direct_members.get(parent) {
            Some(members) => members.iter().collect(),
            None => Vec::new(),
        }
    }

    /* TODO-cleanup should not need to consume "object" */
    pub fn object_lookup_memberships(&self, object: O) -> Vec<&Membership<O>> {
        match self.memberships.get(&Member::Object(object)) {
            Some(memberships) => memberships.iter().collect(),
            None => Vec::new(),
        }
    }

    /* TODO-cleanup should not need to consume "user" */
    pub fn user_lookup_memberships(&self, user: U) -> Vec<&Membership<O>> {
        match self.memberships.get(&Member::User(user)) {
            Some(memberships) => memberships.iter().collect(),
            None => Vec::new(),
        }
    }
}
