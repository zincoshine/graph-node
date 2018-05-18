use futures::sync::oneshot;
use graphql_parser::query;
use serde::ser::*;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::string::FromUtf8Error;

/// A GraphQL query as submitted by a client, either directly or through a subscription.
#[derive(Debug)]
pub struct Query {
    pub document: query::Document,
    pub result_sender: oneshot::Sender<QueryResult>,
}

/// The result of running a query, if successful.
#[derive(Debug, Serialize)]
pub struct QueryResult {
    pub data: Option<HashMap<String, String>>,
    pub errors: Option<Vec<QueryError>>,
}

impl QueryResult {
    pub fn new(data: Option<HashMap<String, String>>) -> Self {
        QueryResult { data, errors: None }
    }

    pub fn add_error(&mut self, e: QueryError) {
        let errors = self.errors.get_or_insert(vec![]);
        errors.push(e);
    }
}

impl From<QueryExecutionError> for QueryResult {
    fn from(e: QueryExecutionError) -> Self {
        let mut result = Self::new(None);
        result.errors = Some(vec![QueryError::from(e)]);
        result
    }
}

/// Error caused while executing a [Query](struct.Query.html).
#[derive(Debug)]
pub enum QueryExecutionError {
    OperationNameRequired,
    OperationNotFound(String),
    NotSupported(String),
    NoRootQueryObjectType,
}

impl Error for QueryExecutionError {
    fn description(&self) -> &str {
        "Query execution error"
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

impl fmt::Display for QueryExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &QueryExecutionError::OperationNameRequired => write!(f, "Operation name required"),
            &QueryExecutionError::OperationNotFound(ref s) => {
                write!(f, "Operation name not found: {}", s)
            }
            &QueryExecutionError::NotSupported(ref s) => write!(f, "Not supported: {}", s),
            &QueryExecutionError::NoRootQueryObjectType => {
                write!(f, "No root Query type defined in the schema")
            }
        }
    }
}

/// Error caused while processing a [Query](struct.Query.html) request.
#[derive(Debug)]
pub enum QueryError {
    EncodingError(FromUtf8Error),
    ParseError(query::ParseError),
    ExecutionError(QueryExecutionError),
}

impl From<FromUtf8Error> for QueryError {
    fn from(e: FromUtf8Error) -> Self {
        QueryError::EncodingError(e)
    }
}

impl From<query::ParseError> for QueryError {
    fn from(e: query::ParseError) -> Self {
        QueryError::ParseError(e)
    }
}

impl From<QueryExecutionError> for QueryError {
    fn from(e: QueryExecutionError) -> Self {
        QueryError::ExecutionError(e)
    }
}

impl Error for QueryError {
    fn description(&self) -> &str {
        "Query error"
    }

    fn cause(&self) -> Option<&Error> {
        match self {
            &QueryError::EncodingError(ref e) => Some(e),
            &QueryError::ExecutionError(ref e) => Some(e),
            _ => None,
        }
    }
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &QueryError::EncodingError(ref e) => write!(f, "{}", e),
            &QueryError::ExecutionError(ref e) => write!(f, "{}", e),
            &QueryError::ParseError(ref e) => write!(f, "{}", e),
        }
    }
}

impl Serialize for QueryError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        let mut msg = format!("{}", self);

        // Serialize parse errors with their location (line, column) to make it easier
        // for users to find where the errors are; this is likely to change as the
        // graphql_parser team makes improvements to their error reporting
        if let &QueryError::ParseError(_) = self {
            // Split the inner message into (first line, rest)
            let inner_msg = msg.replace("query parse error:", "");
            let inner_msg = inner_msg.trim();
            let parts: Vec<&str> = inner_msg.splitn(2, "\n").collect();

            // Find the colon in the first line and split there
            let colon_pos = parts[0].rfind(":").unwrap();
            let (a, b) = parts[0].split_at(colon_pos);

            // Find the line and column numbers and convert them to u32
            let line: u32 = a.matches(char::is_numeric)
                .collect::<String>()
                .parse()
                .unwrap();
            let column: u32 = b.matches(char::is_numeric)
                .collect::<String>()
                .parse()
                .unwrap();

            // Generate the list of locations
            let mut location = HashMap::new();
            location.insert("line", line);
            location.insert("column", column);
            let locations = vec![location];
            map.serialize_entry("locations", &locations)?;

            // Only use the remainder after the location as the error message
            msg = parts[1].to_string();
        }

        map.serialize_entry("message", msg.as_str())?;
        map.end()
    }
}