use crate::align_buffer::AlignedBuffer;
use crate::server::protocol::{Message, Protocol};
use anyhow::Result;
use rkyv::api::high::{HighSerializer, HighValidator};
use rkyv::bytecheck::CheckBytes;
use rkyv::rancor::Error;
use rkyv::ser::allocator::ArenaHandle;
use rkyv::util::AlignedVec;
use rkyv::{access, to_bytes, Archive, Serialize};

#[derive(Archive, Serialize)]
pub(crate) enum RkyvEnvelope<Req, Res> {
    Request { id: u64, payload: Req },
    Response { id: u64, payload: Res },
}

pub trait SerializeBounds:
    Archive
    + Send
    + Sync
    + 'static
    + for<'a> Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, Error>>
{
}

impl<T> SerializeBounds for T where
    T: Archive
        + Send
        + Sync
        + 'static
        + for<'a> Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, Error>>
{
}

pub trait ArchivedBounds:
    for<'a> CheckBytes<HighValidator<'a, Error>> + Send + Sync + 'static
{
}

impl<T> ArchivedBounds for T where
    T: for<'a> CheckBytes<HighValidator<'a, Error>> + Send + Sync + 'static
{
}

pub struct RkyvProtocol<Req, Res> {
    _phantom: std::marker::PhantomData<(Req, Res)>,
}

impl<Req, Res> Protocol for RkyvProtocol<Req, Res>
where
    Req: SerializeBounds,
    for<'a> Req::Archived: ArchivedBounds,
    for<'a> Res: SerializeBounds,
    for<'a> Res::Archived: ArchivedBounds,
    RkyvEnvelope<Req, Res>: Archive,
    for<'a> <RkyvEnvelope<Req, Res> as Archive>::Archived: CheckBytes<HighValidator<'a, Error>>,
    for<'a> RkyvEnvelope<Req, Res>: Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, Error>>,
{
    type Request = Req;
    type Response = Res;

    type RequestView = Req::Archived;
    type ResponseView = Res::Archived;

    fn decode(data: &[u8]) -> Result<Message<&Self::RequestView, &Self::ResponseView>> {
        let archived = access::<ArchivedRkyvEnvelope<Req, Res>, Error>(data)?;

        match archived {
            ArchivedRkyvEnvelope::Request { id, payload } => Ok(Message::Request {
                id: u64::from(id),
                payload,
            }),
            ArchivedRkyvEnvelope::Response { id, payload } => Ok(Message::Response {
                id: u64::from(*id),
                payload,
            }),
        }
    }

    fn encode(
        msg: Message<Self::Request, Self::Response>,
        mut dest: AlignedBuffer,
    ) -> Result<AlignedBuffer> {
        dest.0.clear();

        let envelope = match msg {
            Message::Request { id, payload } => RkyvEnvelope::Request { id, payload },
            Message::Response { id, payload } => RkyvEnvelope::Response { id, payload },
        };

        let bytes = to_bytes::<Error>(&envelope)?;
        dest.0.extend_from_slice(&bytes);
        Ok(dest)
    }
}
