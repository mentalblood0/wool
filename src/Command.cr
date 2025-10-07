require "woollib/common"
require "woollib/Command"

require "./users/Users"

module Wool
  abstract class Command(T)
    mjyd action, add, delete, add_tags, delete_tags, get, get_relations, get_by_tags, add_user

    dc Users, add_user, {u: User}, begin
      s.add **@args
    end

    dc Users, push, {ui: Id, c: Wool::Command(Users) | Wool::Command(Sweater)}, begin
      s.add **@args
    end

    dc Users, pull, {limit: UInt64, from: Id?}, begin
      s.add **@args
    end
  end
end
