extern crate futures;
extern crate graphql_parser;
#[macro_use]
extern crate pretty_assertions;
extern crate graph;
extern crate graph_core;
extern crate graph_graphql;

use futures::sync::oneshot;
use graphql_parser::query as q;
use std::sync::Mutex;

use graph::components::store::EventSource;
use graph::prelude::*;
use graph_graphql::prelude::*;

fn test_schema() -> Schema {
    let mut schema = Schema {
        id: String::from("test-schema"),
        document: api_schema(
            &graphql_parser::parse_schema(
                "
            type Musician {
                id: ID!
                name: String!
                mainBand: Band
                bands: [Band!]!
                writtenSongs: [Song]! @derivedFrom(field: \"writtenBy\")
            }

            type Band {
                id: ID!
                name: String!
                members: [Musician!]! @derivedFrom(field: \"bands\")
            }

            type Song {
                id: ID!
                title: String!
                writtenBy: Musician!
            }
            ",
            ).expect("Test schema invalid"),
        ).expect("Failed to derive API schema from test schema"),
    };
    schema.add_subgraph_id_directives(String::from("test_subgraph"));
    schema
}

#[derive(Clone)]
struct TestStore {
    entities: Vec<Entity>,
}

impl TestStore {
    pub fn new() -> Self {
        TestStore {
            entities: vec![
                Entity::from(vec![
                    ("__typename", Value::from("Musician")),
                    ("id", Value::from("m1")),
                    ("name", Value::from("John")),
                    ("mainBand", Value::from("b1")),
                    (
                        "bands",
                        Value::List(vec![Value::from("b1"), Value::from("b2")]),
                    ),
                ]),
                Entity::from(vec![
                    ("__typename", Value::from("Musician")),
                    ("id", Value::from("m2")),
                    ("name", Value::from("Lisa")),
                    ("mainBand", Value::from("b1")),
                    ("bands", Value::List(vec![Value::from("b1")])),
                ]),
                Entity::from(vec![
                    ("__typename", Value::from("Musician")),
                    ("id", Value::from("m3")),
                    ("name", Value::from("Tom")),
                    ("mainBand", Value::from("b2")),
                    (
                        "bands",
                        Value::List(vec![Value::from("b1"), Value::from("b2")]),
                    ),
                ]),
                Entity::from(vec![
                    ("__typename", Value::from("Musician")),
                    ("id", Value::from("m4")),
                    ("name", Value::from("Valerie")),
                    ("bands", Value::List(vec![])),
                    ("writtenSongs", Value::List(vec![Value::from("s2")])),
                ]),
                Entity::from(vec![
                    ("__typename", Value::from("Band")),
                    ("id", Value::from("b1")),
                    ("name", Value::from("The Musicians")),
                ]),
                Entity::from(vec![
                    ("__typename", Value::from("Band")),
                    ("id", Value::from("b2")),
                    ("name", Value::from("The Amateurs")),
                ]),
                Entity::from(vec![
                    ("__typename", Value::from("Song")),
                    ("id", Value::from("s1")),
                    ("title", Value::from("Cheesy Tune")),
                    ("writtenBy", Value::from("m1")),
                ]),
                Entity::from(vec![
                    ("__typename", Value::from("Song")),
                    ("id", Value::from("s2")),
                    ("title", Value::from("Rock Tune")),
                    ("writtenBy", Value::from("m2")),
                ]),
                Entity::from(vec![
                    ("__typename", Value::from("Song")),
                    ("id", Value::from("s3")),
                    ("title", Value::from("Pop Tune")),
                    ("writtenBy", Value::from("m1")),
                ]),
                Entity::from(vec![
                    ("__typename", Value::from("Song")),
                    ("id", Value::from("s4")),
                    ("title", Value::from("Folk Tune")),
                    ("writtenBy", Value::from("m3")),
                ]),
            ],
        }
    }
}

impl BasicStore for TestStore {
    fn get(&self, key: StoreKey) -> Result<Entity, ()> {
        self.entities
            .iter()
            .find(|entity| {
                entity.get("id") == Some(&Value::String(key.id.clone()))
                    && entity.get("__typename") == Some(&Value::String(key.entity.clone()))
            })
            .map_or(Err(()), |entity| Ok(entity.clone()))
    }

    fn set(&mut self, _key: StoreKey, _entity: Entity, _source: EventSource) -> Result<(), ()> {
        unimplemented!()
    }

    fn delete(&mut self, _key: StoreKey, _source: EventSource) -> Result<(), ()> {
        unimplemented!()
    }

    fn find(&self, query: StoreQuery) -> Result<Vec<Entity>, ()> {
        let entity_name = Value::String(query.entity.clone());

        let entities = self.entities
            .iter()
            .filter(|entity| entity.get("__typename") == Some(&entity_name))
            // We're only supporting the following filters here to to test
            // the filters generated for reference fields and @derivedFrom fields:
            //
            // - And(Contains(...))
            // - And(Equal(...))
            // - And(Or([Equal(...), ...]))
            .filter(|entity| {
                query
                    .filter
                    .as_ref()
                    .and_then(|filter| match filter {
                        StoreFilter::And(filters) => filters.get(0),
                        _ => None,
                    })
                    .map(|filter| match filter {
                        StoreFilter::Equal(k, v) => entity.get(k) == Some(&v),
                        StoreFilter::Contains(k, v) => match entity.get(k) {
                            Some(Value::List(values)) => values.contains(v),
                            _ => false,
                        },
                        StoreFilter::Or(filters) => filters.iter().any(|filter| match filter {
                            StoreFilter::Equal(k,v) => entity.get(k) == Some(&v),
                            _ => unimplemented!(),
                        }),
                        _ => unimplemented!(),
                    })
                    .unwrap_or(true)
            })
            .map(|entity| entity.clone())
            .collect();

        Ok(entities)
    }
}

fn execute_query(query: q::Document) -> QueryResult {
    let (sender, _receiver) = oneshot::channel();

    let query = Query {
        schema: test_schema(),
        document: query,
        variables: None,
        result_sender: sender,
    };

    let logger = Logger::root(slog::Discard, o!());
    let store = Arc::new(Mutex::new(TestStore::new()));
    let store_resolver = StoreResolver::new(&logger, store);

    let options = ExecutionOptions {
        logger: logger,
        resolver: store_resolver,
    };

    execute(&query, options)
}

#[test]
fn can_query_one_to_one_relationship() {
    let result = execute_query(
        graphql_parser::parse_query(
            "
            query {
                musicians {
                    name
                    mainBand {
                        name
                    }
                }
            }
            ",
        ).expect("Invalid test query"),
    );

    assert!(
        result.errors.is_none(),
        format!("Unexpected errors return for query: {:#?}", result.errors)
    );

    assert_eq!(
        result.data,
        Some(object_value(vec![(
            "musicians",
            q::Value::List(vec![
                object_value(vec![
                    ("name", q::Value::String(String::from("John"))),
                    (
                        "mainBand",
                        object_value(vec![(
                            "name",
                            q::Value::String(String::from("The Musicians")),
                        )]),
                    ),
                ]),
                object_value(vec![
                    ("name", q::Value::String(String::from("Lisa"))),
                    (
                        "mainBand",
                        object_value(vec![(
                            "name",
                            q::Value::String(String::from("The Musicians")),
                        )]),
                    ),
                ]),
                object_value(vec![
                    ("name", q::Value::String(String::from("Tom"))),
                    (
                        "mainBand",
                        object_value(vec![(
                            "name",
                            q::Value::String(String::from("The Amateurs")),
                        )]),
                    ),
                ]),
                object_value(vec![
                    ("name", q::Value::String(String::from("Valerie"))),
                    ("mainBand", q::Value::Null),
                ]),
            ]),
        )])),
    )
}

#[test]
fn can_query_one_to_many_relationships_in_both_directions() {
    let result = execute_query(
        graphql_parser::parse_query(
            "
        query {
            musicians {
                name
                writtenSongs {
                    title
                    writtenBy { name }
                }
            }
        }
        ",
        ).expect("Invalid test query"),
    );

    assert!(
        result.errors.is_none(),
        format!("Unexpected errors return for query: {:#?}", result.errors)
    );

    assert_eq!(
        result.data,
        Some(object_value(vec![(
            "musicians",
            q::Value::List(vec![
                object_value(vec![
                    ("name", q::Value::String(String::from("John"))),
                    (
                        "writtenSongs",
                        q::Value::List(vec![
                            object_value(vec![
                                ("title", q::Value::String(String::from("Cheesy Tune"))),
                                (
                                    "writtenBy",
                                    object_value(vec![(
                                        "name",
                                        q::Value::String(String::from("John")),
                                    )]),
                                ),
                            ]),
                            object_value(vec![
                                ("title", q::Value::String(String::from("Pop Tune"))),
                                (
                                    "writtenBy",
                                    object_value(vec![(
                                        "name",
                                        q::Value::String(String::from("John")),
                                    )]),
                                ),
                            ]),
                        ]),
                    ),
                ]),
                object_value(vec![
                    ("name", q::Value::String(String::from("Lisa"))),
                    (
                        "writtenSongs",
                        q::Value::List(vec![object_value(vec![
                            ("title", q::Value::String(String::from("Rock Tune"))),
                            (
                                "writtenBy",
                                object_value(vec![(
                                    "name",
                                    q::Value::String(String::from("Lisa")),
                                )]),
                            ),
                        ])]),
                    ),
                ]),
                object_value(vec![
                    ("name", q::Value::String(String::from("Tom"))),
                    (
                        "writtenSongs",
                        q::Value::List(vec![object_value(vec![
                            ("title", q::Value::String(String::from("Folk Tune"))),
                            (
                                "writtenBy",
                                object_value(vec![("name", q::Value::String(String::from("Tom")))]),
                            ),
                        ])]),
                    ),
                ]),
                object_value(vec![
                    ("name", q::Value::String(String::from("Valerie"))),
                    ("writtenSongs", q::Value::List(vec![])),
                ]),
            ]),
        )])),
    )
}

#[test]
fn can_query_many_to_many_relationship() {
    let result = execute_query(
        graphql_parser::parse_query(
            "
            query {
                musicians {
                    name
                    bands {
                        name
                        members {
                            name
                        }
                    }
                }
            }
            ",
        ).expect("Invalid test query"),
    );

    assert!(
        result.errors.is_none(),
        format!("Unexpected errors return for query: {:#?}", result.errors)
    );

    let the_musicians = object_value(vec![
        ("name", q::Value::String(String::from("The Musicians"))),
        (
            "members",
            q::Value::List(vec![
                object_value(vec![("name", q::Value::String(String::from("John")))]),
                object_value(vec![("name", q::Value::String(String::from("Lisa")))]),
                object_value(vec![("name", q::Value::String(String::from("Tom")))]),
            ]),
        ),
    ]);

    let the_amateurs = object_value(vec![
        ("name", q::Value::String(String::from("The Amateurs"))),
        (
            "members",
            q::Value::List(vec![
                object_value(vec![("name", q::Value::String(String::from("John")))]),
                object_value(vec![("name", q::Value::String(String::from("Tom")))]),
            ]),
        ),
    ]);

    assert_eq!(
        result.data,
        Some(object_value(vec![(
            "musicians",
            q::Value::List(vec![
                object_value(vec![
                    ("name", q::Value::String(String::from("John"))),
                    (
                        "bands",
                        q::Value::List(vec![the_musicians.clone(), the_amateurs.clone()]),
                    ),
                ]),
                object_value(vec![
                    ("name", q::Value::String(String::from("Lisa"))),
                    ("bands", q::Value::List(vec![the_musicians.clone()])),
                ]),
                object_value(vec![
                    ("name", q::Value::String(String::from("Tom"))),
                    (
                        "bands",
                        q::Value::List(vec![the_musicians.clone(), the_amateurs.clone()]),
                    ),
                ]),
                object_value(vec![
                    ("name", q::Value::String(String::from("Valerie"))),
                    ("bands", q::Value::List(vec![])),
                ]),
            ]),
        )]))
    );
}
