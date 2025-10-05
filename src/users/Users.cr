require "yaml"

require "trove"
require "woollib/common"

module Wool
  class Users
    mserializable

    getter chest : Trove::Chest

    def add(u : User)
      raise Exception.new "User #{u.to_json} already exists" if @chest.has_key! u.id, "type"
      @chest.set u.id, "", Wool.to_tj u
    end

    def get(un : User::Name) : User?
      Wool.from_tj User, (@chest.get (User.new un).id).not_nil! rescue nil
    end

    def delete(un : User::Name)
      @chest.transaction do |tx|
        tx.delete (User.new un).id
        tx.where({"integration.user_id" => (User.new un).id.string}) { |ii| tx.delete ii }
      end
    end

    def add(i : Integration)
      raise Exception.new "Integration #{i.to_json} already exists" if @chest.has_key! i.id, "type"
      @chest.set i.id, "", Wool.to_tj i
    end

    def get(s : Site, pseudonym : String) : User?
      ii = @chest.where({"integration.site"      => (String.from_json s.to_json),
                         "integration.pseudonym" => pseudonym}).first rescue return nil
      i = Wool.from_tj Integration, (@chest.get ii).not_nil!
      Wool.from_tj User, (@chest.get i.user_id).not_nil! rescue nil
    end

    def delete(i : Integration)
      @chest.delete i.id
    end

    def add(un : User::Name, c : Wool::Command)
      @chest.push (User.new un).id, "user.queue", Wool.to_tj c
    end
  end
end
