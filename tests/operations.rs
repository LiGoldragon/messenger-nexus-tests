use flow::{Address, Aspect, Model, Topic};
use messenger_core::{
    Configuration, Content, CredentialPath, CryptoStorePath, FlowClientPath, Homeserver,
    LivingUserId, MatrixFailure, Reason, Refusal, RoomId, Subject, Text, ThreadId,
    emission::{
        Deliver_Data, Delivery_Data, Emission, PostInThread_Data, PostRoot_Data, Posting_Data,
        Rooting_Data,
    },
    memory::State,
    meta_signal::Setting,
    operation::{
        Configuring_Data, Observing_Data, Operation, Operation_Hearing_Data,
        Operation_Opening_Data, Operation_Saying_Data, Outcome, Outcome_Hearing_Data,
        Outcome_Opening_Data, Outcome_Saying_Data,
    },
    operations::{
        Attempting, Configuring, Emitting, Hearing, Messenger, Opening, Remembering, Saying,
    },
};

struct CellMemory {
    state: State,
    recall_failure: Option<Refusal>,
    replacement_failure: Option<Refusal>,
}
impl Remembering for CellMemory {
    fn recall(&mut self) -> Result<State, Refusal> {
        match &self.recall_failure {
            Some(refusal) => Err(refusal.clone()),
            None => Ok(self.state.clone()),
        }
    }
    fn replace(&mut self, state: State) -> Result<(), Refusal> {
        match &self.replacement_failure {
            Some(refusal) => Err(refusal.clone()),
            None => {
                self.state = state;
                Ok(())
            }
        }
    }
}

struct RecordingEmission {
    seen: Vec<Emission>,
    rooting: Rooting_Data,
    posting: Posting_Data,
    delivery: Delivery_Data,
}
impl Emitting for RecordingEmission {
    async fn post_root(&mut self, request: PostRoot_Data) -> Rooting_Data {
        self.seen.push(Emission::PostRoot(request));
        self.rooting.clone()
    }
    async fn post_in_thread(&mut self, request: PostInThread_Data) -> Posting_Data {
        self.seen.push(Emission::PostInThread(request));
        self.posting.clone()
    }
    async fn deliver(&mut self, request: Deliver_Data) -> Delivery_Data {
        self.seen.push(Emission::Deliver(request));
        self.delivery.clone()
    }
}

struct Fixture {
    messenger: Messenger<CellMemory, RecordingEmission>,
    opener: Address,
}
impl Default for Fixture {
    fn default() -> Self {
        let opener = Address {
            aspect: Aspect::Mind,
            topic: Topic("subjectOwner".into()),
            model: Model::Sol,
        };
        let communicator = Address {
            aspect: Aspect::Psyche,
            topic: Topic("core".into()),
            model: Model::Fable,
        };
        Self {
            opener,
            messenger: Messenger {
                memory: CellMemory {
                    state: State {
                        thread_vector: Vec::new(),
                        configuration: Configuration {
                            homeserver_option: Some(Homeserver("http://127.0.0.1".into())),
                            room_id_option: Some(RoomId("!isolated:fixture.invalid".into())),
                            living_user_id_option: Some(LivingUserId(
                                "@living:fixture.invalid".into(),
                            )),
                            credential_path_option: Some(CredentialPath("/fixture/session".into())),
                            crypto_store_path_option: Some(CryptoStorePath(
                                "/fixture/crypto".into(),
                            )),
                            address_option: Some(communicator),
                            flow_client_path_option: Some(FlowClientPath("/fixture/flow".into())),
                        },
                    },
                    recall_failure: None,
                    replacement_failure: None,
                },
                emission: RecordingEmission {
                    seen: Vec::new(),
                    rooting: Rooting_Data::Rooted(ThreadId("$root".into())),
                    posting: Posting_Data::Posted,
                    delivery: Delivery_Data::Delivered,
                },
            },
        }
    }
}

#[tokio::test]
async fn open_records_root_subject_and_explicit_owner() {
    let mut fixture = Fixture::default();
    let subject = Subject("One subject".into());
    let result = fixture
        .messenger
        .open(Operation_Opening_Data {
            address: fixture.opener.clone(),
            subject: subject.clone(),
            content: Content::Text(Text("opening".into())),
        })
        .await;
    assert_eq!(
        result,
        Outcome_Opening_Data::Opened(ThreadId("$root".into()))
    );
    let records = &fixture.messenger.memory.state.thread_vector;
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].address, fixture.opener);
    assert_eq!(records[0].subject, subject);
    assert_eq!(fixture.messenger.emission.seen.len(), 1);
}

#[tokio::test]
async fn thread_heard_uses_owner_subject_and_verbatim_text() {
    let mut fixture = Fixture::default();
    fixture
        .messenger
        .open(Operation_Opening_Data {
            address: fixture.opener.clone(),
            subject: Subject("A subject".into()),
            content: Content::Text(Text("root".into())),
        })
        .await;
    fixture.messenger.emission.seen.clear();
    let text = Text("  Original words\n«untouched»  ".into());
    let result = fixture
        .messenger
        .hear(Operation_Hearing_Data {
            thread_id_option: Some(ThreadId("$root".into())),
            content: Content::Text(text.clone()),
        })
        .await;
    assert_eq!(result, Outcome_Hearing_Data::Delivered);
    assert_eq!(
        fixture.messenger.emission.seen,
        vec![Emission::Deliver(Deliver_Data {
            address: fixture.opener,
            subject: Subject("A subject".into()),
            text,
        })]
    );
}

#[tokio::test]
async fn room_level_heard_uses_configured_communicator() {
    let mut fixture = Fixture::default();
    let configuration = fixture.messenger.memory.state.configuration.clone();
    let result = fixture
        .messenger
        .hear(Operation_Hearing_Data {
            thread_id_option: None,
            content: Content::Text(Text("new subject".into())),
        })
        .await;
    assert_eq!(result, Outcome_Hearing_Data::Delivered);
    assert_eq!(
        fixture.messenger.emission.seen,
        vec![Emission::Deliver(Deliver_Data {
            address: configuration.address_option.unwrap(),
            subject: Subject(configuration.room_id_option.unwrap().0),
            text: Text("new subject".into()),
        })]
    );
}

#[tokio::test]
async fn unknown_thread_is_refused_without_an_external_call() {
    let mut fixture = Fixture::default();
    let result = fixture
        .messenger
        .hear(Operation_Hearing_Data {
            thread_id_option: Some(ThreadId("$unknown".into())),
            content: Content::Text(Text("reply".into())),
        })
        .await;
    assert_eq!(
        result,
        Outcome_Hearing_Data::Refused(Refusal::UnknownThread(ThreadId("$unknown".into())))
    );
    assert!(fixture.messenger.emission.seen.is_empty());
}

#[tokio::test]
async fn failed_root_does_not_create_a_thread_record() {
    let mut fixture = Fixture::default();
    fixture.messenger.emission.rooting = Rooting_Data::Unrooted(MatrixFailure::Forbidden);
    let result = fixture
        .messenger
        .open(Operation_Opening_Data {
            address: fixture.opener,
            subject: Subject("subject".into()),
            content: Content::Text(Text("root".into())),
        })
        .await;
    assert_eq!(
        result,
        Outcome_Opening_Data::Refused(Refusal::MatrixFailure(MatrixFailure::Forbidden))
    );
    assert!(fixture.messenger.memory.state.thread_vector.is_empty());
}

#[tokio::test]
async fn failed_record_commit_is_not_answered_as_opened() {
    let mut fixture = Fixture::default();
    let refusal = Refusal::Memory(Reason("fixture write failure".into()));
    fixture.messenger.memory.replacement_failure = Some(refusal.clone());
    let result = fixture
        .messenger
        .open(Operation_Opening_Data {
            address: fixture.opener,
            subject: Subject("subject".into()),
            content: Content::Text(Text("root".into())),
        })
        .await;
    assert_eq!(result, Outcome_Opening_Data::Refused(refusal));
    assert!(fixture.messenger.memory.state.thread_vector.is_empty());
    assert_eq!(fixture.messenger.emission.seen.len(), 1);
}

#[tokio::test]
async fn images_are_refused_before_any_external_call() {
    let mut fixture = Fixture::default();
    let result = fixture
        .messenger
        .open(Operation_Opening_Data {
            address: fixture.opener,
            subject: Subject("subject".into()),
            content: Content::Image(messenger_core::FilePath("/fixture/image".into())),
        })
        .await;
    assert_eq!(
        result,
        Outcome_Opening_Data::Refused(Refusal::UnsupportedContent)
    );
    assert!(fixture.messenger.emission.seen.is_empty());
}

#[test]
fn configuration_changes_only_the_selected_value() {
    let mut fixture = Fixture::default();
    let before = fixture.messenger.memory.state.configuration.clone();
    let result = fixture
        .messenger
        .configure(Setting::RoomId(RoomId("!new:fixture.invalid".into())));
    assert_eq!(result, Configuring_Data::Configured);
    let mut expected = before;
    expected.room_id_option = Some(RoomId("!new:fixture.invalid".into()));
    assert_eq!(fixture.messenger.memory.state.configuration, expected);
}

#[tokio::test]
async fn observation_failure_keeps_its_own_attempt_kind() {
    let mut fixture = Fixture::default();
    let refusal = Refusal::Memory(Reason("fixture read failure".into()));
    fixture.messenger.memory.recall_failure = Some(refusal.clone());
    let outcome = fixture.messenger.attempt(Operation::Observing).await;
    assert_eq!(
        outcome,
        Outcome::Observing(Observing_Data::Refused(refusal))
    );
}

#[tokio::test]
async fn saying_to_unknown_root_does_not_post() {
    let mut fixture = Fixture::default();
    let result = fixture
        .messenger
        .say(Operation_Saying_Data {
            thread_id: ThreadId("$unknown".into()),
            content: Content::Text(Text("reply".into())),
        })
        .await;
    assert_eq!(
        result,
        Outcome_Saying_Data::Refused(Refusal::UnknownThread(ThreadId("$unknown".into())))
    );
    assert!(fixture.messenger.emission.seen.is_empty());
}

#[tokio::test]
async fn changing_communicator_changes_room_destination_but_keeps_thread_owner() {
    let mut fixture = Fixture::default();
    fixture
        .messenger
        .open(Operation_Opening_Data {
            address: fixture.opener.clone(),
            subject: Subject("Existing subject".into()),
            content: Content::Text(Text("root".into())),
        })
        .await;
    let communicator = Address {
        aspect: Aspect::Psyche,
        topic: Topic("core".into()),
        model: Model::Opus,
    };
    assert_eq!(
        fixture
            .messenger
            .configure(Setting::Communicator(communicator.clone())),
        Configuring_Data::Configured
    );
    fixture.messenger.emission.seen.clear();
    for thread_id_option in [None, Some(ThreadId("$root".into()))] {
        assert_eq!(
            fixture
                .messenger
                .hear(Operation_Hearing_Data {
                    thread_id_option,
                    content: Content::Text(Text("words".into())),
                })
                .await,
            Outcome_Hearing_Data::Delivered
        );
    }
    let room = fixture
        .messenger
        .memory
        .state
        .configuration
        .room_id_option
        .clone()
        .unwrap();
    assert_eq!(
        fixture.messenger.emission.seen,
        vec![
            Emission::Deliver(Deliver_Data {
                address: communicator,
                subject: Subject(room.0),
                text: Text("words".into()),
            }),
            Emission::Deliver(Deliver_Data {
                address: fixture.opener,
                subject: Subject("Existing subject".into()),
                text: Text("words".into()),
            }),
        ]
    );
}

#[tokio::test]
async fn room_message_requires_a_configured_communicator() {
    let mut fixture = Fixture::default();
    fixture.messenger.memory.state.configuration.address_option = None;
    assert_eq!(
        fixture
            .messenger
            .hear(Operation_Hearing_Data {
                thread_id_option: None,
                content: Content::Text(Text("words".into())),
            })
            .await,
        Outcome_Hearing_Data::Refused(Refusal::Unconfigured)
    );
    assert!(fixture.messenger.emission.seen.is_empty());
}
