//! Converts between cnp and native IndraDB models

use crate::autogen;
use capnp::Error as CapnpError;
use indradb;
use serde_json;
use std::fmt::Display;
use std::vec::IntoIter;

pub fn map_capnp_err<T, E: Display>(result: Result<T, E>) -> Result<T, capnp::Error> {
    result.map_err(|err| capnp::Error::failed(format!("{}", err)))
}

pub fn from_vertex<'a>(vertex: &indradb::Vertex, mut builder: autogen::vertex::Builder<'a>) {
    builder.set_id(vertex.id);
    builder.set_t(&vertex.t.0);
}

pub fn to_vertex<'a>(reader: &autogen::vertex::Reader<'a>) -> Result<indradb::Vertex, CapnpError> {
    let id = reader.get_id();
    let t = map_capnp_err(indradb::Type::new(reader.get_t()?))?;
    Ok(indradb::Vertex::new(id, t))
}

pub fn from_edge<'a>(edge: &indradb::Edge, mut builder: autogen::edge::Builder<'a>) {
    builder.set_outbound_id(edge.outbound_id);
    builder.set_t(&edge.t.0);
    builder.set_inbound_id(edge.inbound_id);
}

pub fn to_edge<'a>(reader: &autogen::edge::Reader<'a>) -> Result<indradb::Edge, CapnpError> {
    let outbound_id = reader.get_outbound_id();
    let t = map_capnp_err(indradb::Type::new(reader.get_t()?))?;
    let inbound_id = reader.get_inbound_id();
    Ok(indradb::Edge::new(outbound_id, t, inbound_id))
}

pub fn from_vertex_property<'a>(
    property: &indradb::VertexProperty,
    mut builder: autogen::vertex_property::Builder<'a>,
) {
    builder.set_id(property.id);
    builder.set_value(&property.value.to_string());
}

pub fn to_vertex_property<'a>(
    reader: &autogen::vertex_property::Reader<'a>,
) -> Result<indradb::VertexProperty, CapnpError> {
    let id = reader.get_id();
    let value = map_capnp_err(serde_json::from_str(reader.get_value()?))?;
    Ok(indradb::VertexProperty::new(id, value))
}

pub fn from_vertex_properties<'a>(
    properties: &indradb::VertexProperties,
    builder: &mut autogen::vertex_properties::Builder<'a>,
) {
    from_vertex(&properties.vertex, builder.reborrow().init_vertex());
    let mut props_builder = builder.reborrow().init_props(properties.props.len() as u32);
    for (i, prop) in properties.props.iter().enumerate() {
        from_named_property(prop, props_builder.reborrow().get(i as u32));
    }
}

pub fn to_vertex_properties<'a>(
    reader: &autogen::vertex_properties::Reader<'a>,
) -> Result<indradb::VertexProperties, CapnpError> {
    let vertex = map_capnp_err(to_vertex(&reader.get_vertex()?))?;
    let named_props: Result<Vec<indradb::NamedProperty>, CapnpError> =
        reader.get_props()?.into_iter().map(to_named_property).collect();
    Ok(indradb::VertexProperties::new(vertex, named_props?))
}

pub fn from_named_property<'a>(property: &indradb::NamedProperty, mut builder: autogen::property::Builder<'a>) {
    builder.set_name(&property.name);
    builder.set_value(&property.value.to_string());
}

pub fn to_named_property(reader: autogen::property::Reader) -> Result<indradb::NamedProperty, CapnpError> {
    let name = map_capnp_err(reader.get_name())?.to_string();
    let value = map_capnp_err(serde_json::from_str(reader.get_value()?))?;
    Ok(indradb::NamedProperty::new(name, value))
}

pub fn from_edge_properties<'a>(
    properties: &indradb::EdgeProperties,
    builder: &mut autogen::edge_properties::Builder<'a>,
) {
    from_edge(&properties.edge, builder.reborrow().init_edge());
    let mut props_builder = builder.reborrow().init_props(properties.props.len() as u32);
    for (i, prop) in properties.props.iter().enumerate() {
        from_named_property(prop, props_builder.reborrow().get(i as u32));
    }
}

pub fn to_edge_properties<'a>(
    reader: &autogen::edge_properties::Reader<'a>,
) -> Result<indradb::EdgeProperties, CapnpError> {
    let edge = map_capnp_err(to_edge(&reader.get_edge()?))?;
    let named_props: Result<Vec<indradb::NamedProperty>, CapnpError> =
        reader.get_props()?.into_iter().map(to_named_property).collect();
    Ok(indradb::EdgeProperties::new(edge, named_props?))
}

pub fn from_edge_property<'a>(property: &indradb::EdgeProperty, mut builder: autogen::edge_property::Builder<'a>) {
    builder.set_value(&property.value.to_string());
    from_edge(&property.edge, builder.init_edge());
}

pub fn to_edge_property<'a>(reader: &autogen::edge_property::Reader<'a>) -> Result<indradb::EdgeProperty, CapnpError> {
    let edge = to_edge(&reader.get_edge()?)?;
    let value = map_capnp_err(serde_json::from_str(reader.get_value()?))?;
    Ok(indradb::EdgeProperty::new(edge, value))
}

pub fn from_vertex_query<'a>(q: &indradb::VertexQuery, builder: autogen::vertex_query::Builder<'a>) {
    match q {
        indradb::VertexQuery::Range(q) => {
            let mut builder = builder.init_range();

            if let Some(start_id) = q.start_id {
                builder.set_start_id(start_id);
            }

            if let Some(ref t) = q.t {
                builder.set_t(&t.0);
            }

            builder.set_limit(q.limit);
        }
        indradb::VertexQuery::Specific(q) => {
            let mut builder = builder.init_specific().init_ids(q.ids.len() as u32);

            for (i, id) in q.ids.iter().enumerate() {
                builder.set(i as u32, *id);
            }
        }
        indradb::VertexQuery::Pipe(q) => {
            let mut builder = builder.init_pipe();
            builder.set_direction(from_edge_direction(q.direction));
            builder.set_limit(q.limit);

            if let Some(ref t) = q.t {
                builder.set_t(&t.0);
            }

            from_edge_query(&q.inner, builder.init_inner());
        }
    }
}

pub fn to_vertex_query<'a>(reader: &autogen::vertex_query::Reader<'a>) -> Result<indradb::VertexQuery, CapnpError> {
    match reader.which()? {
        autogen::vertex_query::Range(params) => {
            let start_id = params.get_start_id();
            let t_str = params.get_t()?;
            let mut range = indradb::RangeVertexQuery::new(params.get_limit());

            if start_id != 0 {
                range = range.start_id(start_id);
            }

            if t_str != "" {
                range = range.t(map_capnp_err(indradb::Type::new(t_str))?);
            }

            Ok(range.into())
        }
        autogen::vertex_query::Specific(params) => {
            let ids: Vec<u64> = params
                .get_ids()?
                .into_iter()
                .collect();
            Ok(indradb::SpecificVertexQuery::new(ids).into())
        }
        autogen::vertex_query::Pipe(params) => {
            let inner = Box::new(to_edge_query(&params.get_inner()?)?);
            let direction = to_edge_direction(params.get_direction()?);
            let limit = params.get_limit();
            let t_str = params.get_t()?;
            let mut pipe = indradb::PipeVertexQuery::new(inner, direction, limit);

            if t_str != "" {
                pipe = pipe.t(map_capnp_err(indradb::Type::new(t_str))?);
            }

            Ok(pipe.into())
        }
    }
}

pub fn from_vertex_property_query<'a>(
    q: &indradb::VertexPropertyQuery,
    mut builder: autogen::vertex_property_query::Builder<'a>,
) {
    builder.set_name(&q.name);
    from_vertex_query(&q.inner, builder.init_inner());
}

pub fn to_vertex_property_query<'a>(
    reader: &autogen::vertex_property_query::Reader<'a>,
) -> Result<indradb::VertexPropertyQuery, CapnpError> {
    let inner = to_vertex_query(&reader.get_inner()?)?;
    let name = reader.get_name()?;
    Ok(indradb::VertexPropertyQuery::new(inner, name))
}

pub fn from_edge_query<'a>(q: &indradb::EdgeQuery, builder: autogen::edge_query::Builder<'a>) {
    match q {
        indradb::EdgeQuery::Specific(specific) => {
            let mut builder = builder.init_specific().init_edges(specific.edges.len() as u32);

            for (i, edge) in specific.edges.iter().enumerate() {
                from_edge(edge, builder.reborrow().get(i as u32));
            }
        }
        indradb::EdgeQuery::Pipe(pipe) => {
            let mut builder = builder.init_pipe();
            builder.set_direction(from_edge_direction(pipe.direction));

            if let Some(t) = &pipe.t {
                builder.set_t(&t.0);
            }

            builder.set_limit(pipe.limit);
            from_vertex_query(&pipe.inner, builder.init_inner());
        }
    }
}

pub fn to_edge_query<'a>(reader: &autogen::edge_query::Reader<'a>) -> Result<indradb::EdgeQuery, CapnpError> {
    match reader.which()? {
        autogen::edge_query::Specific(params) => {
            let keys: Result<Vec<indradb::Edge>, CapnpError> = params
                .get_edges()?
                .into_iter()
                .map(|reader| to_edge(&reader))
                .collect();
            Ok(indradb::EdgeQuery::Specific(indradb::SpecificEdgeQuery::new(keys?)))
        }
        autogen::edge_query::Pipe(params) => {
            let inner = Box::new(to_vertex_query(&params.get_inner()?)?);
            let direction = to_edge_direction(params.get_direction()?);
            let limit = params.get_limit();
            let mut pipe = indradb::PipeEdgeQuery::new(inner, direction, limit);

            let t = params.get_t()?;
            if t != "" {
                pipe = pipe.t(map_capnp_err(indradb::Type::new(t))?);
            }

            Ok(indradb::EdgeQuery::Pipe(pipe))
        }
    }
}

pub fn from_edge_property_query<'a>(
    q: &indradb::EdgePropertyQuery,
    mut builder: autogen::edge_property_query::Builder<'a>,
) {
    builder.set_name(&q.name);
    from_edge_query(&q.inner, builder.init_inner());
}

pub fn to_edge_property_query<'a>(
    reader: &autogen::edge_property_query::Reader<'a>,
) -> Result<indradb::EdgePropertyQuery, CapnpError> {
    let inner = to_edge_query(&reader.get_inner()?)?;
    let name = reader.get_name()?;
    Ok(indradb::EdgePropertyQuery::new(inner, name))
}

pub fn from_bulk_insert_items<'a>(
    items: &[indradb::BulkInsertItem],
    mut builder: capnp::struct_list::Builder<'a, autogen::bulk_insert_item::Owned>,
) -> Result<(), CapnpError> {
    for (i, item) in items.iter().enumerate() {
        let builder = builder.reborrow().get(i as u32);

        match item {
            indradb::BulkInsertItem::Vertex(t) => {
                let mut builder = builder.init_vertex();
                builder.set_t(&t.0);
            }
            indradb::BulkInsertItem::Edge(edge) => {
                let builder = builder.init_edge();
                from_edge(edge, builder.get_edge()?);
            }
            indradb::BulkInsertItem::VertexProperty(id, name, value) => {
                let mut builder = builder.init_vertex_property();
                builder.set_id(*id);
                builder.set_name(name);
                builder.set_value(&value.to_string());
            }
            indradb::BulkInsertItem::EdgeProperty(key, name, value) => {
                let mut builder = builder.init_edge_property();
                builder.set_name(name);
                builder.set_value(&value.to_string());
                from_edge(key, builder.get_edge()?);
            }
        }
    }

    Ok(())
}

pub fn to_bulk_insert_items<'a>(
    reader: &capnp::struct_list::Reader<'a, autogen::bulk_insert_item::Owned>,
) -> Result<IntoIter<indradb::BulkInsertItem>, CapnpError> {
    let items: Result<Vec<indradb::BulkInsertItem>, CapnpError> = reader
        .into_iter()
        .map(|item| match item.which()? {
            autogen::bulk_insert_item::Vertex(params) => {
                let t = map_capnp_err(indradb::Type::new(params.get_t()?))?;
                Ok(indradb::BulkInsertItem::Vertex(t))
            }
            autogen::bulk_insert_item::Edge(params) => {
                let edge_key = to_edge(&params.get_edge()?)?;
                Ok(indradb::BulkInsertItem::Edge(edge_key))
            }
            autogen::bulk_insert_item::VertexProperty(params) => {
                let id = params.get_id();
                let name = params.get_name()?.to_string();
                let value = map_capnp_err(serde_json::from_str(params.get_value()?))?;
                Ok(indradb::BulkInsertItem::VertexProperty(id, name, value))
            }
            autogen::bulk_insert_item::EdgeProperty(params) => {
                let key = to_edge(&params.get_edge()?)?;
                let name = params.get_name()?.to_string();
                let value = map_capnp_err(serde_json::from_str(params.get_value()?))?;
                Ok(indradb::BulkInsertItem::EdgeProperty(key, name, value))
            }
        })
        .collect();
    Ok(items?.into_iter())
}

pub fn from_edge_direction(direction: indradb::EdgeDirection) -> autogen::EdgeDirection {
    match direction {
        indradb::EdgeDirection::Outbound => autogen::EdgeDirection::Outbound,
        indradb::EdgeDirection::Inbound => autogen::EdgeDirection::Inbound,
    }
}

pub fn to_edge_direction(direction: autogen::EdgeDirection) -> indradb::EdgeDirection {
    match direction {
        autogen::EdgeDirection::Outbound => indradb::EdgeDirection::Outbound,
        autogen::EdgeDirection::Inbound => indradb::EdgeDirection::Inbound,
    }
}
