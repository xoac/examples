use async_graphql::{dynamic::*, Value, ID};
use futures_util::StreamExt;

use crate::{simple_broker::SimpleBroker, Book, Storage};

pub fn schema() -> Result<Schema, SchemaError> {
    let book = Object::new("Book")
        .description("A book that will be stored.")
        .field(
            Field::new("id", TypeRef::named_nn(TypeRef::ID), |ctx| {
                FieldFuture::new(async move {
                    let book = ctx.parent_value.try_downcast_ref::<Book>()?;
                    Ok(Some(Value::from(book.id.to_owned())))
                })
            })
            .description("The id of the book."),
        )
        .field(
            Field::new("name", TypeRef::named_nn(TypeRef::STRING), |ctx| {
                FieldFuture::new(async move {
                    let book = ctx.parent_value.try_downcast_ref::<Book>()?;
                    Ok(Some(Value::from(book.name.to_owned())))
                })
            })
            .description("The name of the book."),
        )
        .field(
            Field::new("author", TypeRef::named_nn(TypeRef::STRING), |ctx| {
                FieldFuture::new(async move {
                    let book = ctx.parent_value.try_downcast_ref::<Book>()?;
                    Ok(Some(Value::from(book.author.to_owned())))
                })
            })
            .description("The author of the book."),
        );

    let query = Object::new("Query")
        .field(
            Field::new("getBook", TypeRef::named(book.type_name()), |ctx| {
                FieldFuture::new(async move {
                    let id = ctx.args.try_get("id")?;
                    let book_id = match id.string() {
                        Ok(id) => id.to_string(),
                        Err(_) => id.u64()?.to_string(),
                    };
                    let book_id = book_id.parse::<usize>()?;
                    let store = ctx.data_unchecked::<Storage>().lock().await;
                    let book = store.get(book_id).cloned();
                    Ok(book.map(FieldValue::owned_any))
                })
            })
            .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::ID))),
        )
        .field(Field::new(
            "getBooks",
            TypeRef::named_nn_list_nn(book.type_name()),
            |ctx| {
                FieldFuture::new(async move {
                    let store = ctx.data_unchecked::<Storage>().lock().await;
                    let books: Vec<Book> = store.iter().map(|(_, book)| book.clone()).collect();
                    Ok(Some(FieldValue::list(
                        books.into_iter().map(FieldValue::owned_any),
                    )))
                })
            },
        ));

    let mutatation = Object::new("Mutation").field(
        Field::new("createBook", TypeRef::named(TypeRef::ID), |ctx| {
            FieldFuture::new(async move {
                let mut store = ctx.data_unchecked::<Storage>().lock().await;
                let name = ctx.args.try_get("name")?;
                let author = ctx.args.try_get("author")?;
                let entry = store.vacant_entry();
                let id: ID = entry.key().into();
                let book = Book {
                    id: id.clone(),
                    name: name.string()?.to_string(),
                    author: author.string()?.to_string(),
                };
                entry.insert(book.clone());
                SimpleBroker::publish(book.clone());
                Ok(Some(Value::from(id)))
            })
        })
        .argument(InputValue::new("name", TypeRef::named_nn(TypeRef::STRING)))
        .argument(InputValue::new(
            "author",
            TypeRef::named_nn(TypeRef::STRING),
        )),
    );
    // .field(
    //     Field::new("updateValue", TypeRef::named_nn(TypeRef::INT), |ctx| {
    //         FieldFuture::new(async move {
    //             let mut store_value = ctx.data_unchecked::<BookStore>().value.lock().await;
    //             let new_value = ctx.args.try_get("value")?;
    //             let value = new_value.u64()?;
    //             *store_value = value;
    //             Ok(Some(Value::from(*store_value)))
    //         })
    //     })
    //     .argument(InputValue::new("value", TypeRef::named_nn(TypeRef::INT))),
    // );
    // Todo:Show book.value
    let subscription = Subscription::new("Subscription").field(SubscriptionField::new(
        "bookAdded",
        TypeRef::named_nn(book.type_name()),
        |_| {
            SubscriptionFieldFuture::new(async {
                Ok(SimpleBroker::<Book>::subscribe().map(|book| Ok(FieldValue::owned_any(book))))
            })
        },
    ));

    Schema::build(
        query.type_name(),
        Some(mutatation.type_name()),
        Some(subscription.type_name()),
    )
    .register(book)
    .register(query)
    .register(subscription)
    .register(mutatation)
    .data(Storage::default())
    .finish()
}
