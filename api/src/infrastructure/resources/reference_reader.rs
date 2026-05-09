use std::io::Read;

use serde::de::{self, Deserializer, SeqAccess, Visitor};

use crate::domain::models::reference::ReferenceEntry;
use crate::shared::error::AppError;

pub fn for_each_reference_entry<R, F>(reader: R, mut on_entry: F) -> Result<(), AppError>
where
    R: Read,
    F: FnMut(ReferenceEntry) -> Result<(), AppError>,
{
    struct ArrayVisitor<'a, F>(&'a mut F);

    impl<'de, F> Visitor<'de> for ArrayVisitor<'_, F>
    where
        F: FnMut(ReferenceEntry) -> Result<(), AppError>,
    {
        type Value = ();

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a JSON array of reference entries")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            while let Some(entry) = seq.next_element::<ReferenceEntry>()? {
                (self.0)(entry).map_err(de::Error::custom)?;
            }
            Ok(())
        }
    }

    let mut deserializer = serde_json::Deserializer::from_reader(reader);
    deserializer.deserialize_any(ArrayVisitor(&mut on_entry))?;
    Ok(())
}
