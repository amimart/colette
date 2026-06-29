#![allow(dead_code)]

use colette::collection::{collection, Collection};
use colette::entity::Entity;
use colette::error::CodecError;
use colette::impl_enum_key;
use colette::index::{Index, Multi, Unique};
use colette::index_registry::{Cons, Nil};
use colette::key::{Key, KeySize};
use colette::store::MultiStore;

pub fn run_collection_contract_tests<DB: MultiStore>(make_db: impl Fn() -> DB) {
    single_value_primary_key_behaviour(&make_db);
    tuple_primary_key_behaviour(&make_db);
}

pub fn single_value_primary_key_behaviour<DB: MultiStore>(make_db: &impl Fn() -> DB) {
    let users = user_collection("single_value_primary_key_behaviour", make_db());
    let ada = user(
        100,
        "ada",
        "ada@example.test",
        Region::Europe,
        AccountStatus::Active,
        Plan::Team,
        "core",
        1,
    );

    users.insert(&ada).unwrap();

    assert_eq!(users.get(100).unwrap(), Some(ada));
    assert_eq!(users.get(404).unwrap(), None);
}

pub fn tuple_primary_key_behaviour<DB: MultiStore>(make_db: &impl Fn() -> DB) {
    let memberships = membership_collection("tuple_primary_key_behaviour", make_db());
    let core_ada = membership("core", 100, Role::Owner, "founder");

    memberships.insert(&core_ada).unwrap();

    assert_eq!(
        memberships.get(("core".to_string(), 100)).unwrap(),
        Some(core_ada)
    );
    assert_eq!(memberships.get(("core".to_string(), 404)).unwrap(), None);
}

pub type UserCollection<DB> = Collection<
    DB,
    User,
    Cons<
        ByTeamStatusSeat,
        Cons<
            ByRegionStatus,
            Cons<
                ByStatus,
                Cons<UniqueRegionPlanHandle, Cons<UniqueRegionHandle, Cons<UniqueEmail, Nil>>>,
            >,
        >,
    >,
>;

pub type MembershipCollection<DB> = Collection<DB, Membership, Nil>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    Americas,
    Europe,
    Pacific,
}

impl_enum_key!(Region as u8 {
    Region::Americas => 0,
    Region::Europe => 1,
    Region::Pacific => 2,
});

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountStatus {
    Invited,
    Active,
    Suspended,
}

impl_enum_key!(AccountStatus as u8 {
    AccountStatus::Invited => 0,
    AccountStatus::Active => 1,
    AccountStatus::Suspended => 2,
});

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan {
    Free,
    Team,
    Enterprise,
}

impl_enum_key!(Plan as u8 {
    Plan::Free => 0,
    Plan::Team => 1,
    Plan::Enterprise => 2,
});

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Owner,
    Maintainer,
    Viewer,
}

impl_enum_key!(Role as u8 {
    Role::Owner => 0,
    Role::Maintainer => 1,
    Role::Viewer => 2,
});

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User {
    pub id: u64,
    pub handle: String,
    pub email: String,
    pub region: Region,
    pub status: AccountStatus,
    pub plan: Plan,
    pub team: String,
    pub seat: u16,
}

impl Entity for User {
    type Key<'a> = u64;

    fn key(&self) -> Self::Key<'_> {
        self.id
    }

    fn to_bytes(&self) -> Result<Vec<u8>, CodecError> {
        Ok(format!(
            "{}|{}|{}|{}|{}|{}|{}|{}",
            self.id,
            self.handle,
            self.email,
            self.region.as_str(),
            self.status.as_str(),
            self.plan.as_str(),
            self.team,
            self.seat
        )
        .into_bytes())
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, CodecError> {
        let text = std::str::from_utf8(bytes).unwrap();
        let fields = text.split('|').collect::<Vec<_>>();
        assert_eq!(fields.len(), 8, "malformed user fixture: {text}");

        Ok(Self {
            id: fields[0].parse().unwrap(),
            handle: fields[1].to_string(),
            email: fields[2].to_string(),
            region: Region::parse(fields[3]),
            status: AccountStatus::parse(fields[4]),
            plan: Plan::parse(fields[5]),
            team: fields[6].to_string(),
            seat: fields[7].parse().unwrap(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Membership {
    pub org: String,
    pub user_id: u64,
    pub role: Role,
    pub label: String,
}

impl Entity for Membership {
    type Key<'a> = (&'a str, u64);

    fn key(&self) -> Self::Key<'_> {
        (self.org.as_str(), self.user_id)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, CodecError> {
        Ok(format!(
            "{}|{}|{}|{}",
            self.org,
            self.user_id,
            self.role.as_str(),
            self.label
        )
        .into_bytes())
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, CodecError> {
        let text = std::str::from_utf8(bytes).unwrap();
        let fields = text.split('|').collect::<Vec<_>>();
        assert_eq!(fields.len(), 4, "malformed membership fixture: {text}");

        Ok(Self {
            org: fields[0].to_string(),
            user_id: fields[1].parse().unwrap(),
            role: Role::parse(fields[2]),
            label: fields[3].to_string(),
        })
    }
}

pub struct UniqueEmail;
pub struct UniqueRegionHandle;
pub struct UniqueRegionPlanHandle;
pub struct ByStatus;
pub struct ByRegionStatus;
pub struct ByTeamStatusSeat;

impl Index<User> for UniqueEmail {
    type Key<'a> = &'a str;
    type Kind<'a> = Unique;
    const NAME: &'static str = "unique_email";

    fn key(entity: &User) -> Self::Key<'_> {
        entity.email.as_str()
    }
}

impl Index<User> for UniqueRegionHandle {
    type Key<'a> = (Region, &'a str);
    type Kind<'a> = Unique;
    const NAME: &'static str = "unique_region_handle";

    fn key(entity: &User) -> Self::Key<'_> {
        (entity.region, entity.handle.as_str())
    }
}

impl Index<User> for UniqueRegionPlanHandle {
    type Key<'a> = (Region, Plan, &'a str);
    type Kind<'a> = Unique;
    const NAME: &'static str = "unique_region_plan_handle";

    fn key(entity: &User) -> Self::Key<'_> {
        (entity.region, entity.plan, entity.handle.as_str())
    }
}

impl Index<User> for ByStatus {
    type Key<'a> = (AccountStatus,);
    type Kind<'a> = Multi;
    const NAME: &'static str = "by_status";

    fn key(entity: &User) -> Self::Key<'_> {
        (entity.status,)
    }
}

impl Index<User> for ByRegionStatus {
    type Key<'a> = (Region, AccountStatus);
    type Kind<'a> = Multi;
    const NAME: &'static str = "by_region_status";

    fn key(entity: &User) -> Self::Key<'_> {
        (entity.region, entity.status)
    }
}

impl Index<User> for ByTeamStatusSeat {
    type Key<'a> = (&'a str, AccountStatus, u16);
    type Kind<'a> = Multi;
    const NAME: &'static str = "by_team_status_seat";

    fn key(entity: &User) -> Self::Key<'_> {
        (entity.team.as_str(), entity.status, entity.seat)
    }
}

pub fn user_collection<DB: MultiStore>(name: &'static str, db: DB) -> UserCollection<DB> {
    db.prepare(
        name,
        [
            "__main",
            UniqueEmail::NAME,
            UniqueRegionHandle::NAME,
            UniqueRegionPlanHandle::NAME,
            ByStatus::NAME,
            ByRegionStatus::NAME,
            ByTeamStatusSeat::NAME,
        ],
    )
    .unwrap();

    collection::<User, DB>(name, db)
        .with_index::<UniqueEmail>()
        .with_index::<UniqueRegionHandle>()
        .with_index::<UniqueRegionPlanHandle>()
        .with_index::<ByStatus>()
        .with_index::<ByRegionStatus>()
        .with_index::<ByTeamStatusSeat>()
        .build()
}

pub fn membership_collection<DB: MultiStore>(
    name: &'static str,
    db: DB,
) -> MembershipCollection<DB> {
    db.prepare(name, ["__main"]).unwrap();
    collection::<Membership, DB>(name, db).build()
}

pub fn user(
    id: u64,
    handle: &str,
    email: &str,
    region: Region,
    status: AccountStatus,
    plan: Plan,
    team: &str,
    seat: u16,
) -> User {
    User {
        id,
        handle: handle.to_string(),
        email: email.to_string(),
        region,
        status,
        plan,
        team: team.to_string(),
        seat,
    }
}

pub fn membership(org: &str, user_id: u64, role: Role, label: &str) -> Membership {
    Membership {
        org: org.to_string(),
        user_id,
        role,
        label: label.to_string(),
    }
}

impl Region {
    fn as_str(self) -> &'static str {
        match self {
            Self::Americas => "americas",
            Self::Europe => "europe",
            Self::Pacific => "pacific",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "americas" => Self::Americas,
            "europe" => Self::Europe,
            "pacific" => Self::Pacific,
            _ => panic!("unknown region: {value}"),
        }
    }
}

impl AccountStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Invited => "invited",
            Self::Active => "active",
            Self::Suspended => "suspended",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "invited" => Self::Invited,
            "active" => Self::Active,
            "suspended" => Self::Suspended,
            _ => panic!("unknown account status: {value}"),
        }
    }
}

impl Plan {
    fn as_str(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Team => "team",
            Self::Enterprise => "enterprise",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "free" => Self::Free,
            "team" => Self::Team,
            "enterprise" => Self::Enterprise,
            _ => panic!("unknown plan: {value}"),
        }
    }
}

impl Role {
    fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Maintainer => "maintainer",
            Self::Viewer => "viewer",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "owner" => Self::Owner,
            "maintainer" => Self::Maintainer,
            "viewer" => Self::Viewer,
            _ => panic!("unknown role: {value}"),
        }
    }
}
