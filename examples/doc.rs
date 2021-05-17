//! Example configuration (and demo) from the Zanzibar paper that appears to
//! describe the authorization behavior of Google Docs

use miniz::MiniZ;

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
struct ObjectId(&'static str);
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
struct UserId(&'static str);

fn main() {
    /*
     * The following block constructs a MiniZ instance with the same
     * configuration as what's described in Figure 1 in the Zanzibar paper.
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
     * Now demo it.  We'll construct this hierarchy of objects (defined by the
     * "parent" relationship).
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
    assert!(!miniz.set_contains_object_directly(&set_parent, &dir1, doc456));
    /* Contents of "dir2" */
    assert!(miniz.set_contains_object_directly(&set_parent, &dir2, doc456));
    assert!(!miniz.set_contains_object_directly(&set_parent, &dir2, doc123));
    /* Non-existent document is contained nowhere. */
    assert!(!miniz.set_contains_object_directly(&set_parent, &dir2, doc789));
    assert!(!miniz.set_contains_object_directly(&set_parent, &dir2, doc789));
    /* Non-existent set contains nothing */
    assert!(!miniz.set_contains_object_directly(&set_parent, &dir3, doc123));
    assert!(!miniz.set_contains_object_directly(&set_parent, &dir3, doc123));

    /* User associations (direct associations) for "dir1" */
    assert_eq!(
        miniz.set_list_direct_members(&set_owner, &dir1),
        vec![&miniz::Member::User(user_alice)]
    );
    assert!(miniz.set_contains_user_directly(&set_owner, &dir1, user_alice));
    assert!(!miniz.set_contains_user_directly(&set_owner, &dir1, user_bob));
    assert!(!miniz.set_contains_user_directly(&set_owner, &dir1, user_carol));

    assert_eq!(
        miniz.set_list_direct_members(&set_editor, &dir1),
        vec![&miniz::Member::User(user_bob)]
    );
    assert!(!miniz.set_contains_user_directly(&set_editor, &dir1, user_alice));
    assert!(miniz.set_contains_user_directly(&set_editor, &dir1, user_bob));
    assert!(!miniz.set_contains_user_directly(&set_editor, &dir1, user_carol));

    assert_eq!(
        miniz.set_list_direct_members(&set_viewer, &dir1),
        vec![&miniz::Member::User(user_carol)]
    );
    assert!(!miniz.set_contains_user_directly(&set_viewer, &dir1, user_alice));
    assert!(!miniz.set_contains_user_directly(&set_viewer, &dir1, user_bob));
    assert!(miniz.set_contains_user_directly(&set_viewer, &dir1, user_carol));

    /* Reverse indexes */
    assert_eq!(
        miniz.object_lookup_memberships(dir1),
        Vec::new() as Vec<&miniz::Membership<ObjectId>>,
    );
    assert_eq!(
        miniz.object_lookup_memberships(dir2),
        Vec::new() as Vec<&miniz::Membership<ObjectId>>,
    );
    assert_eq!(
        miniz.object_lookup_memberships(doc123),
        vec![&miniz::Membership {
            set_id: set_parent.clone(),
            object: dir1.clone(),
        }]
    );
    assert_eq!(
        miniz.user_lookup_memberships(user_alice),
        vec![&miniz::Membership {
            set_id: set_owner,
            object: dir1.clone(),
        }]
    );
}
