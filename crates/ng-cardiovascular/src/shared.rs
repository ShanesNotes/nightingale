use ng_contract::*;

pub(crate) fn drive<'b, 'k, E>(
    k: &mut Kernel<'b, 'k, Cardiovascular, E>,
    pressure: &ReadPort<'b, Cardiovascular, PerSite<Blood, Pressure>>,
    head: &Parameter<'b, Cardiovascular, Quotient<Pressure, Length>>,
) -> Expr<'b, 'k, Cardiovascular, Pressure>
where
    E: EdgeKind<From = Blood, To = Blood>,
{
    let (up, down) = (k.upstream(pressure), k.downstream(pressure));
    let difference = k.subtract(up, down);
    let per_length = k.parameter(head);
    let fall = k.fall();
    let product = k.multiply(per_length, fall);
    let gravity = k.as_dimension(product);
    k.add(difference, gravity)
}
