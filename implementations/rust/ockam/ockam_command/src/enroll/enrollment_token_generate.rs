use anyhow::{anyhow, Context};
use clap::Args;

use ockam::TcpTransport;
use ockam_api::cloud::enroll::enrollment_token::Attribute;
use ockam_multiaddr::MultiAddr;

use crate::old::identity::load_or_create_identity;
use crate::util::{embedded_node, multiaddr_to_route};
use crate::IdentityOpts;

#[derive(Clone, Debug, Args)]
pub struct GenerateEnrollmentTokenCommand {
    /// Ockam's cloud address
    #[clap(display_order = 1000)]
    address: MultiAddr,

    #[clap(display_order = 1001, long, default_value = "default")]
    vault: String,

    #[clap(display_order = 1002, long, default_value = "default")]
    identity: String,

    #[clap(display_order = 1003, long)]
    overwrite: bool,

    /// Comma-separated list of attributes
    #[clap(display_order = 1004, last = true, required = true)]
    attributes: Vec<String>,
}

impl<'a> From<&'a GenerateEnrollmentTokenCommand> for IdentityOpts {
    fn from(other: &'a GenerateEnrollmentTokenCommand) -> Self {
        Self {
            overwrite: other.overwrite,
        }
    }
}

impl GenerateEnrollmentTokenCommand {
    pub fn run(command: GenerateEnrollmentTokenCommand) {
        embedded_node(generate, command);
    }
}

async fn generate(
    mut ctx: ockam::Context,
    command: GenerateEnrollmentTokenCommand,
) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // TODO: The identity below will be used to create a secure channel when cloud nodes support it.
    let identity = load_or_create_identity(&IdentityOpts::from(&command), &ctx).await?;

    let route = multiaddr_to_route(&command.address)
        .ok_or_else(|| anyhow!("failed to parse address: {}", command.address))?;

    let attributes: Vec<Attribute> = command
        .attributes
        .iter()
        .map(|kv| {
            let mut s = kv.split(',');
            let k = s.next().context(format!(
                "failed to parse key from pair: {kv:?}. Expected a \"key,value\" pair."
            ))?;
            let v = s
                .next()
                .context(format!("no value found on pair: {kv:?}"))?;
            if k.is_empty() {
                anyhow::bail!("attribute name can't be empty at pair {kv:?}")
            } else if v.is_empty() {
                anyhow::bail!("attribute value can't be empty at pair {kv:?}")
            } else {
                Ok(Attribute::new(k, v))
            }
        })
        .collect::<anyhow::Result<Vec<Attribute>>>()?;

    let mut api_client = ockam_api::cloud::MessagingClient::new(route, &ctx).await?;
    let token = api_client
        .generate_enrollment_token(&identity.id.to_string(), &attributes)
        .await?;
    println!("Token generated successfully: {:?}", token.token);

    ctx.stop().await?;
    Ok(())
}
