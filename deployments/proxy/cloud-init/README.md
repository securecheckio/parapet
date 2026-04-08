# Cloud-Init Configurations

VM initialization scripts for automated server provisioning.

## Files

### `cloud-init-docker.yaml` (Recommended)

Docker-based deployment with automatic setup:

- Installs Docker
- Pulls and runs parapet container
- Configures firewall and HTTPS
- Sets up log rotation
- **Best for:** Easy updates, portability

### `cloud-init-native.yaml` (Max Performance)

Native binary deployment:

- Downloads pre-built binaries
- Installs as systemd service
- Direct hardware access
- **Best for:** High-throughput production (>5000 req/s)

### `cloud-init-legacy.yaml`

Legacy configuration for backwards compatibility.

## Usage with Terraform

These files are automatically used by Terraform:

```hcl
# In terraform.tfvars
deployment_mode = "docker"  # Uses cloud-init-docker.yaml
# OR
deployment_mode = "native"  # Uses cloud-init-native.yaml
```

## Manual Usage

Can be used directly with cloud providers:

```bash
# DigitalOcean
doctl compute droplet create parapet \
  --image ubuntu-24-04-x64 \
  --size s-1vcpu-1gb \
  --region nyc3 \
  --user-data-file cloud-init-docker.yaml

# AWS
aws ec2 run-instances \
  --image-id ami-xxx \
  --instance-type t3.micro \
  --user-data file://cloud-init-docker.yaml
```

## Configuration

Cloud-init scripts pull configuration from:

- Terraform variables (when using terraform)
- Environment variables set during VM creation
- Default values in the scripts

## Troubleshooting

Check cloud-init logs on the VM:

```bash
# View cloud-init output
sudo cat /var/log/cloud-init-output.log

# Check for errors
sudo journalctl -u cloud-init
```

