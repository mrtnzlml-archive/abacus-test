use crate::auth::users::AnyUser;
use crate::graphql::AbacusGraphQLResult;
use crate::graphql_context::Context;
use juniper::{EmptySubscription, FieldResult, RootNode};

#[derive(Clone, Copy, Debug)]
pub struct Query;

/// Unfortunately, due to Rust limitations, it's not possible to implement custom queries per crate.
/// Therefore, we have to have one main "god" query which exposes the public interface. Good side of
/// this is that it forces us to make these resolvers as thin as possible and moving all the business
/// logic lower in the app structure.
#[juniper::graphql_object(
    context = Context,
    name = "Query",
    description = "Root query of the graph.",
)]
impl Query {
    /// Returns information about the current user (can be authenticated or anonymous).
    async fn whoami(context: &Context) -> crate::auth::api::WhoamiPayload {
        crate::auth::api::whoami(context).await
    }

    async fn list_users(context: &Context) -> AbacusGraphQLResult<Vec<AnyUser>> {
        Ok(crate::auth::api::list_users(&context).await?)
    }

    fn commerce() -> crate::commerce::api::CommerceQuery {
        crate::commerce::api::CommerceQuery {}
    }

    fn menu() -> crate::menu::api::MenuQuery {
        crate::menu::api::MenuQuery {}
    }

    fn pos() -> crate::pos::api::POSQuery {
        crate::pos::api::POSQuery {}
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Mutation;

#[juniper::graphql_object(
    context = Context,
    name = "Mutation",
    description = "Root mutation of the graph.",
)]
impl Mutation {
    /// This function accepts Google ID token (after receiving it from Google Sign-In in a webapp)
    /// and returns authorization payload. There is no concept of sign-in and sign-up: every
    /// whitelisted user with a valid JWT ID token will be authorized. Invalid tokens and users
    /// that are not whitelisted will be rejected.
    ///
    /// Repeated calls will result in a new session token and deauthorization of the previous
    /// token (if it exist). Original session token is returned back only once and cannot be
    /// retrieved later (it's irreversibly hashed in the database).
    async fn authorize_webapp(
        google_id_token: String,
        context: &Context,
    ) -> FieldResult<crate::auth::api::AuthorizeWebappPayload> {
        crate::auth::api::authorize_webapp(&google_id_token, &context).await
    }

    /// The purpose of this `deauthorize` mutation is to remove the active sessions and effectively
    /// make the mobile application/webapp unsigned. Applications should remove the session token
    /// once de-authorized.
    ///
    /// Repeated calls will result in failure since it's not possible to deauthorize twice.
    async fn deauthorize(
        session_token: String, // TODO: this could be removed (?) - we can use the user from context
        context: &Context,
    ) -> crate::auth::api::DeauthorizePayload {
        crate::auth::api::deauthorize(&session_token, &context).await
    }

    fn commerce() -> crate::commerce::api::CommerceMutation {
        crate::commerce::api::CommerceMutation {}
    }

    fn pos() -> crate::pos::api::POSMutation {
        crate::pos::api::POSMutation {}
    }
}

pub type Schema = RootNode<'static, Query, Mutation, EmptySubscription<Context>>;

pub fn create_graphql_schema() -> Schema {
    Schema::new(Query, Mutation, EmptySubscription::<Context>::new())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    fn test_graphql_schema_snapshot(schema: &str, new_schema_path: &str, saved_schema_path: &str) {
        let saved_schema_snapshot =
            fs::read_to_string(saved_schema_path).expect("unable to read schema file");

        assert!(signedsource::is_signed(&saved_schema_snapshot));
        assert!(signedsource::is_valid_signature(&saved_schema_snapshot));

        let new_schema_snapshot =
            signedsource::sign_file(&format!("# {}\n\n{}", signedsource::SIGNING_TOKEN, schema));

        if saved_schema_snapshot != new_schema_snapshot {
            fs::write(new_schema_path, new_schema_snapshot)
                .expect("unable to write new schema file");
        }

        assert_eq!(
            Path::new(new_schema_path).exists(),
            false,
            "schema snapshot with *.new extension should not exist - please resolve it"
        );
    }

    /// This test will make sure that the schema generated by server is ready to be used by
    /// public clients. Please note that the paths below are relative to the project root (not this
    /// file, not the whole monorepo)
    #[test]
    fn schema_snapshot_test() {
        let new_schema_path = "./schema.graphql.new";
        let saved_schema_path = "./schema.graphql";
        test_graphql_schema_snapshot(
            &super::create_graphql_schema().as_schema_language(),
            &new_schema_path,
            &saved_schema_path,
        );
    }
}
