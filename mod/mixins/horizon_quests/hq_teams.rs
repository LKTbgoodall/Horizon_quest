// Teams (parties): who plays together and shares one progress. A player in
// no party is their own team, whose id is their UUID string, so a personal
// progress file is the same before and after teams existed. A party has an
// id of its own (`party-<16 hex>`), a name players type, an owner, members
// and invitations. The registry is kept in `<world>/horizon-quests/teams.json`.
// Progress follows FTB Quests' rules, reimplemented here: creating a party
// copies the creator's progress into it, joining merges the joiner's progress
// into the party's, and leaving gives the player back the personal progress
// they had, with every reward they claimed in the party still claimed.
// Pure: no game types.
use super::hq_model::{FORMAT_VERSION, Progress};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The longest party name.
pub const NAME_MAX: usize = 16;

/// Every party of a world.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Teams {
    pub version: u32,
    /// Party id → party.
    pub parties: BTreeMap<String, Party>,
}

impl Default for Teams {
    fn default() -> Self {
        Self { version: FORMAT_VERSION, parties: BTreeMap::new() }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Party {
    pub name: String,
    /// The owner's player id (a member too).
    pub owner: String,
    /// Player id → their name when last seen.
    pub members: BTreeMap<String, String>,
    /// Player id → name: those invited who have not joined yet.
    pub invites: BTreeMap<String, String>,
    /// Unix seconds.
    pub created: u64,
}

/// Why a team request was refused: what to tell the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    BadName,
    NameTaken,
    AlreadyInParty,
    NotInParty,
    NotOwner,
    NoSuchParty,
    NotInvited,
    AlreadyMember,
    AlreadyInvited,
    NotMember,
    Yourself,
}

impl Refusal {
    pub fn text(self) -> &'static str {
        match self {
            Refusal::BadName => "A party name is 1 to 16 letters, digits, `_` or `-`.",
            Refusal::NameTaken => "A party already has that name.",
            Refusal::AlreadyInParty => "You are already in a party: leave it first (/quests team leave).",
            Refusal::NotInParty => "You are not in a party.",
            Refusal::NotOwner => "Only the party's owner can do that.",
            Refusal::NoSuchParty => "There is no party with that name.",
            Refusal::NotInvited => "That party has not invited you.",
            Refusal::AlreadyMember => "That player is already in your party.",
            Refusal::AlreadyInvited => "That player is already invited.",
            Refusal::NotMember => "That player is not in your party.",
            Refusal::Yourself => "You cannot do that to yourself.",
        }
    }
}

/// What a player leaving (or being removed from) a party did to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Departure {
    pub party: String,
    /// Nobody is left: the party is gone.
    pub disbanded: bool,
    /// The owner left: the member who owns it now.
    pub new_owner: Option<String>,
}

/// Whether `name` can name a party.
pub fn valid_name(name: &str) -> bool {
    (1..=NAME_MAX).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

impl Teams {
    /// The party player `player` is a member of.
    pub fn party_of(&self, player: &str) -> Option<&str> {
        self.parties.iter().find(|(_, party)| party.members.contains_key(player)).map(|(id, _)| id.as_str())
    }

    /// The team whose progress player `player` plays on: their party, else
    /// their own (their player id).
    pub fn team_of(&self, player: &str) -> String {
        self.party_of(player).unwrap_or(player).to_owned()
    }

    /// The party named `name`, ignoring case.
    pub fn find(&self, name: &str) -> Option<&str> {
        self.parties.iter().find(|(_, party)| party.name.eq_ignore_ascii_case(name)).map(|(id, _)| id.as_str())
    }

    pub fn party(&self, id: &str) -> Option<&Party> {
        self.parties.get(id)
    }

    /// The parties that invited player `player`: (id, name).
    pub fn invitations(&self, player: &str) -> Vec<(&str, &str)> {
        self.parties
            .iter()
            .filter(|(_, party)| party.invites.contains_key(player))
            .map(|(id, party)| (id.as_str(), party.name.as_str()))
            .collect()
    }

    /// A new party named `name`, owned by `owner` (named `owner_name`). Its
    /// id is made from `seed`; another seed is drawn while it is taken.
    pub fn create(&mut self, owner: &str, owner_name: &str, name: &str, now: u64, seed: &mut dyn FnMut() -> u64) -> Result<String, Refusal> {
        if !valid_name(name) {
            return Err(Refusal::BadName);
        }
        if self.find(name).is_some() {
            return Err(Refusal::NameTaken);
        }
        if self.party_of(owner).is_some() {
            return Err(Refusal::AlreadyInParty);
        }
        let id = loop {
            let id = format!("party-{:016x}", seed());
            if !self.parties.contains_key(&id) {
                break id;
            }
        };
        let members = BTreeMap::from([(owner.to_owned(), owner_name.to_owned())]);
        let party = Party { name: name.to_owned(), owner: owner.to_owned(), members, invites: BTreeMap::new(), created: now };
        self.parties.insert(id.clone(), party);
        Ok(id)
    }

    /// The owner of `by`'s party invites player `target` (named
    /// `target_name`): the party's id.
    pub fn invite(&mut self, by: &str, target: &str, target_name: &str) -> Result<String, Refusal> {
        if by == target {
            return Err(Refusal::Yourself);
        }
        let id = self.party_of(by).ok_or(Refusal::NotInParty)?.to_owned();
        let party = self.parties.get_mut(&id).ok_or(Refusal::NotInParty)?;
        if party.owner != by {
            return Err(Refusal::NotOwner);
        }
        if party.members.contains_key(target) {
            return Err(Refusal::AlreadyMember);
        }
        if party.invites.insert(target.to_owned(), target_name.to_owned()).is_some() {
            return Err(Refusal::AlreadyInvited);
        }
        Ok(id)
    }

    /// Whether player `player` may join the party named `name` now: its id.
    pub fn can_join(&self, player: &str, name: &str) -> Result<String, Refusal> {
        let id = self.find(name).ok_or(Refusal::NoSuchParty)?;
        if self.party_of(player).is_some() {
            return Err(Refusal::AlreadyInParty);
        }
        let party = &self.parties[id];
        if !party.invites.contains_key(player) {
            return Err(Refusal::NotInvited);
        }
        Ok(id.to_owned())
    }

    /// Player `player` (named `player_name`) accepts the invitation of the
    /// party named `name`: its id.
    pub fn join(&mut self, player: &str, player_name: &str, name: &str) -> Result<String, Refusal> {
        let id = self.can_join(player, name)?;
        if let Some(party) = self.parties.get_mut(&id) {
            party.invites.remove(player);
            party.members.insert(player.to_owned(), player_name.to_owned());
        }
        Ok(id)
    }

    /// Player `player` leaves their party.
    pub fn leave(&mut self, player: &str) -> Result<Departure, Refusal> {
        let id = self.party_of(player).ok_or(Refusal::NotInParty)?.to_owned();
        Ok(self.remove(&id, player))
    }

    /// The owner of `by`'s party removes member `target` from it.
    pub fn kick(&mut self, by: &str, target: &str) -> Result<Departure, Refusal> {
        if by == target {
            return Err(Refusal::Yourself);
        }
        let id = self.party_of(by).ok_or(Refusal::NotInParty)?.to_owned();
        let party = &self.parties[&id];
        if party.owner != by {
            return Err(Refusal::NotOwner);
        }
        if !party.members.contains_key(target) {
            return Err(Refusal::NotMember);
        }
        Ok(self.remove(&id, target))
    }

    /// The member named `name` (ignoring case) of `by`'s party: their id.
    pub fn member_named(&self, by: &str, name: &str) -> Option<String> {
        let party = self.parties.get(self.party_of(by)?)?;
        party.members.iter().find(|(_, n)| n.eq_ignore_ascii_case(name)).map(|(id, _)| id.clone())
    }

    /// The name a member is shown under, as they were last seen: whether it
    /// changed.
    pub fn saw(&mut self, player: &str, name: &str) -> bool {
        let mut changed = false;
        for party in self.parties.values_mut() {
            if let Some(known) = party.members.get_mut(player)
                && known != name
            {
                *known = name.to_owned();
                changed = true;
            }
        }
        changed
    }

    fn remove(&mut self, id: &str, player: &str) -> Departure {
        let mut departure = Departure { party: id.to_owned(), disbanded: false, new_owner: None };
        let Some(party) = self.parties.get_mut(id) else {
            return departure;
        };
        party.members.remove(player);
        if party.members.is_empty() {
            self.parties.remove(id);
            departure.disbanded = true;
        } else if party.owner == player {
            // The member who has the lowest id: the same answer every time.
            let next = party.members.keys().next().cloned().unwrap_or_default();
            party.owner.clone_from(&next);
            departure.new_owner = Some(next);
        }
        departure
    }
}

/// Joining a party: what `from` (the joiner's progress) has done is added to
/// `into` (the party's). A quest or a task counts as done in the party when
/// either side did it; claims add up; the earliest time is kept.
pub fn merge_progress(into: &mut Progress, from: &Progress) {
    let earliest = |map: &mut BTreeMap<String, u64>, other: &BTreeMap<String, u64>| {
        for (key, &time) in other {
            map.entry(key.clone()).and_modify(|t| *t = (*t).min(time)).or_insert(time);
        }
    };
    earliest(&mut into.started, &from.started);
    earliest(&mut into.completed, &from.completed);
    for (task, &value) in &from.tasks {
        let known = into.tasks.entry(task.clone()).or_insert(0);
        *known = (*known).max(value);
    }
    for (reward, players) in &from.claimed {
        into.claimed.entry(reward.clone()).or_default().extend(players.iter().cloned());
    }
    for (quest, &times) in &from.repeats {
        let known = into.repeats.entry(quest.clone()).or_insert(0);
        *known = (*known).max(times);
    }
    into.advancements.extend(from.advancements.iter().cloned());
    into.book_given |= from.book_given;
}

/// Leaving a party: every reward player `player` claimed in `party` stays
/// claimed in their own progress `personal`.
pub fn keep_claims(personal: &mut Progress, party: &Progress, player: &str) {
    for (reward, players) in &party.claimed {
        if players.contains(player) {
            personal.claimed.entry(reward.clone()).or_default().insert(player.to_owned());
        }
    }
    personal.book_given |= party.book_given;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeds() -> impl FnMut() -> u64 {
        let mut next = 0;
        move || {
            next += 1;
            next
        }
    }

    #[test]
    fn a_party_is_created_joined_on_invitation_and_left() {
        let mut teams = Teams::default();
        let mut seed = seeds();
        assert_eq!(teams.team_of("steve"), "steve");
        let id = teams.create("steve", "Steve", "Miners", 10, &mut seed).unwrap();
        assert_eq!(id, "party-0000000000000001");
        assert_eq!(teams.team_of("steve"), id);
        assert_eq!(teams.find("miners"), Some(id.as_str()));

        // Alex needs an invitation, from the owner.
        assert_eq!(teams.join("alex", "Alex", "Miners"), Err(Refusal::NotInvited));
        assert_eq!(teams.invite("alex", "steve", "Steve"), Err(Refusal::NotInParty));
        assert_eq!(teams.invite("steve", "steve", "Steve"), Err(Refusal::Yourself));
        assert_eq!(teams.invite("steve", "alex", "Alex"), Ok(id.clone()));
        assert_eq!(teams.invite("steve", "alex", "Alex"), Err(Refusal::AlreadyInvited));
        assert_eq!(teams.invitations("alex"), [(id.as_str(), "Miners")]);
        assert_eq!(teams.join("alex", "Alex", "Nobody"), Err(Refusal::NoSuchParty));
        assert_eq!(teams.join("alex", "Alex", "MINERS"), Ok(id.clone()));
        assert!(teams.invitations("alex").is_empty());
        assert_eq!(teams.team_of("alex"), id);
        assert_eq!(teams.invite("steve", "alex", "Alex"), Err(Refusal::AlreadyMember));
        assert_eq!(teams.invite("alex", "kim", "Kim"), Err(Refusal::NotOwner));
        assert_eq!(teams.member_named("steve", "ALEX").as_deref(), Some("alex"));

        // The owner leaves: Alex owns it; Alex leaves: it is gone.
        let left = teams.leave("steve").unwrap();
        assert_eq!(left, Departure { party: id.clone(), disbanded: false, new_owner: Some("alex".into()) });
        assert_eq!(teams.team_of("steve"), "steve");
        assert_eq!(teams.party(&id).unwrap().owner, "alex");
        assert_eq!(teams.leave("steve"), Err(Refusal::NotInParty));
        assert!(teams.leave("alex").unwrap().disbanded);
        assert!(teams.parties.is_empty());
    }

    #[test]
    fn names_ids_and_one_party_each() {
        let mut teams = Teams::default();
        let mut seed = || 7;
        for bad in ["", "two words", "seventeen_letters", "é"] {
            assert_eq!(teams.create("steve", "Steve", bad, 0, &mut seed), Err(Refusal::BadName), "{bad}");
        }
        let first = teams.create("steve", "Steve", "A-team_1", 0, &mut seed).unwrap();
        assert_eq!(teams.create("alex", "Alex", "a-TEAM_1", 0, &mut seed), Err(Refusal::NameTaken));
        assert_eq!(teams.create("steve", "Steve", "Other", 0, &mut seed), Err(Refusal::AlreadyInParty));
        // A taken id draws another seed.
        let mut seeds = [7, 7, 8].into_iter();
        let second = teams.create("alex", "Alex", "Other", 0, &mut || seeds.next().unwrap()).unwrap();
        assert_ne!(first, second);
        assert_eq!(second, "party-0000000000000008");

        // In a party already: no joining another.
        teams.invite("alex", "steve", "Steve").unwrap();
        assert_eq!(teams.join("steve", "Steve", "Other"), Err(Refusal::AlreadyInParty));
    }

    #[test]
    fn the_owner_kicks_members() {
        let mut teams = Teams::default();
        let id = teams.create("steve", "Steve", "Miners", 0, &mut seeds()).unwrap();
        teams.invite("steve", "alex", "Alex").unwrap();
        teams.join("alex", "Alex", "Miners").unwrap();
        assert_eq!(teams.kick("alex", "steve"), Err(Refusal::NotOwner));
        assert_eq!(teams.kick("steve", "steve"), Err(Refusal::Yourself));
        assert_eq!(teams.kick("steve", "kim"), Err(Refusal::NotMember));
        assert_eq!(teams.kick("steve", "alex"), Ok(Departure { party: id, disbanded: false, new_owner: None }));
        assert_eq!(teams.team_of("alex"), "alex");
        assert_eq!(teams.kick("kim", "alex"), Err(Refusal::NotInParty));
    }

    #[test]
    fn names_follow_the_members() {
        let mut teams = Teams::default();
        let id = teams.create("steve", "Steve", "Miners", 0, &mut seeds()).unwrap();
        assert!(teams.saw("steve", "Steve2"));
        assert!(!teams.saw("steve", "Steve2"));
        assert!(!teams.saw("alex", "Alex"));
        assert_eq!(teams.party(&id).unwrap().members["steve"], "Steve2");
        assert_eq!(teams.party(&id).unwrap().members.len(), 1);
    }

    #[test]
    fn the_registry_round_trips_through_json() {
        let mut teams = Teams::default();
        teams.create("steve", "Steve", "Miners", 5, &mut seeds()).unwrap();
        let text = serde_json::to_string(&teams).unwrap();
        assert_eq!(serde_json::from_str::<Teams>(&text).unwrap(), teams);
        assert_eq!(serde_json::from_str::<Teams>("{}").unwrap(), Teams::default());
    }

    #[test]
    fn joining_merges_and_leaving_keeps_claims() {
        let mut party = Progress::default();
        party.started.insert("a/q".into(), 50);
        party.completed.insert("a/q".into(), 60);
        party.tasks.insert("a/q/t".into(), 3);
        party.claimed.entry("a/q/r".into()).or_default().insert("steve".into());
        party.repeats.insert("a/again".into(), 1);

        let mut alex = Progress::default();
        alex.started.insert("a/q".into(), 40);
        alex.started.insert("a/other".into(), 70);
        alex.tasks.insert("a/q/t".into(), 2);
        alex.tasks.insert("a/other/t".into(), 5);
        alex.claimed.entry("a/q/r".into()).or_default().insert("alex".into());
        alex.repeats.insert("a/again".into(), 3);
        alex.advancements.insert("minecraft:story/mine_stone".into());
        alex.book_given = true;

        merge_progress(&mut party, &alex);
        assert_eq!(party.started["a/q"], 40);
        assert_eq!(party.started["a/other"], 70);
        assert_eq!(party.completed["a/q"], 60);
        assert_eq!((party.task("a/q/t"), party.task("a/other/t")), (3, 5));
        assert!(party.has_claimed("a/q/r", "steve") && party.has_claimed("a/q/r", "alex"));
        assert_eq!(party.repeats["a/again"], 3);
        assert!(party.advancements.contains("minecraft:story/mine_stone") && party.book_given);

        // Steve leaves: his own progress gets back what he claimed in the party, nothing else.
        party.claimed.entry("a/z/r".into()).or_default().insert("steve".into());
        let mut steve = Progress::default();
        keep_claims(&mut steve, &party, "steve");
        assert!(steve.has_claimed("a/q/r", "steve") && steve.has_claimed("a/z/r", "steve"));
        assert!(!steve.has_claimed("a/q/r", "alex"));
        assert!(steve.completed.is_empty() && steve.tasks.is_empty());
    }
}
