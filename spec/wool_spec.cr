require "spec"

require "../src/users/User"
require "../src/users/Users"

alias Config = {users: Wool::Users}

describe Wool do
  config = Config.from_yaml File.read "spec/config.yml"
  users = config[:users]

  it "can add/delete users" do
    u = Wool::Users::User.new (Wool::Users::User::Name.new "name"), Wool::Users::User::Role::User
    users.add u
    (users.get u.name).should eq u
    users.delete u.name
    (users.get u.name).should eq nil
  end
end
